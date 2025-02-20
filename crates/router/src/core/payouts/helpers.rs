use api_models::{enums, payment_methods::Card, payouts};
use common_utils::{
    crypto::Encryptable,
    encryption::Encryption,
    errors::CustomResult,
    ext_traits::{AsyncExt, StringExt},
    fp_utils, id_type, payout_method_utils as payout_additional, pii, type_name,
    types::{
        keymanager::{Identifier, KeyManagerState},
        MinorUnit, UnifiedCode, UnifiedMessage,
    },
};
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use common_utils::{generate_customer_id_of_default_length, types::keymanager::ToEncryptable};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::type_encryption::{crypto_operation, CryptoOperation};
use masking::{ExposeInterface, PeekInterface, Secret, SwitchStrategy};
use router_env::logger;

use super::PayoutData;
#[cfg(feature = "payouts")]
use crate::core::payments::route_connector_v1_for_payouts;
use crate::{
    consts,
    core::{
        errors::{self, RouterResult, StorageErrorExt},
        payment_methods::{
            cards,
            transformers::{DataDuplicationCheck, StoreCardReq, StoreGenericReq, StoreLockerReq},
            vault,
        },
        payments::{helpers as payment_helpers, routing, CustomerDetails},
        routing::TransactionData,
        utils as core_utils,
    },
    db::StorageInterface,
    routes::{metrics, SessionState},
    services,
    types::{
        api::{self, enums as api_enums},
        domain::{self, types::AsyncLift},
        storage,
        transformers::ForeignFrom,
    },
    utils::{self, OptionExt},
};

#[allow(clippy::too_many_arguments)]
pub async fn make_payout_method_data(
    state: &SessionState,
    payout_method_data: Option<&api::PayoutMethodData>,
    payout_token: Option<&str>,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    payout_type: Option<api_enums::PayoutType>,
    merchant_key_store: &domain::MerchantKeyStore,
    payout_data: Option<&mut PayoutData>,
    storage_scheme: storage::enums::MerchantStorageScheme,
) -> RouterResult<Option<api::PayoutMethodData>> {
    let db = &*state.store;
    let hyperswitch_token = if let Some(payout_token) = payout_token {
        if payout_token.starts_with("temporary_token_") {
            Some(payout_token.to_string())
        } else {
            let certain_payout_type = payout_type.get_required_value("payout_type")?.to_owned();
            let key = format!(
                "pm_token_{}_{}_hyperswitch",
                payout_token,
                api_enums::PaymentMethod::foreign_from(certain_payout_type)
            );

            let redis_conn = state
                .store
                .get_redis_conn()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get redis connection")?;

            let hyperswitch_token = redis_conn
                .get_key::<Option<String>>(&key.into())
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to fetch the token from redis")?
                .ok_or(error_stack::Report::new(
                    errors::ApiErrorResponse::UnprocessableEntity {
                        message: "Token is invalid or expired".to_owned(),
                    },
                ))?;
            let payment_token_data = hyperswitch_token
                .clone()
                .parse_struct("PaymentTokenData")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("failed to deserialize hyperswitch token data")?;

            let payment_token = match payment_token_data {
                storage::PaymentTokenData::PermanentCard(storage::CardTokenData {
                    locker_id,
                    token,
                    ..
                }) => locker_id.or(Some(token)),
                storage::PaymentTokenData::TemporaryGeneric(storage::GenericTokenData {
                    token,
                }) => Some(token),
                _ => None,
            };
            payment_token.or(Some(payout_token.to_string()))
        }
    } else {
        None
    };

    match (
        payout_method_data.to_owned(),
        hyperswitch_token,
        payout_data,
    ) {
        // Get operation
        (None, Some(payout_token), _) => {
            if payout_token.starts_with("temporary_token_")
                || payout_type == Some(api_enums::PayoutType::Bank)
            {
                let (pm, supplementary_data) = vault::Vault::get_payout_method_data_from_temporary_locker(
                    state,
                    &payout_token,
                    merchant_key_store,
                )
                .await
                .attach_printable(
                    "Payout method for given token not found or there was a problem fetching it",
                )?;
                utils::when(
                    supplementary_data
                        .customer_id
                        .ne(&Some(customer_id.to_owned())),
                    || {
                        Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payout method and customer passed in payout are not same".into() })
                    },
                )?;
                Ok(pm)
            } else {
                let resp = cards::get_card_from_locker(
                    state,
                    customer_id,
                    merchant_id,
                    payout_token.as_ref(),
                )
                .await
                .attach_printable("Payout method [card] could not be fetched from HS locker")?;
                Ok(Some({
                    api::PayoutMethodData::Card(api::CardPayout {
                        card_number: resp.card_number,
                        expiry_month: resp.card_exp_month,
                        expiry_year: resp.card_exp_year,
                        card_holder_name: resp.name_on_card,
                    })
                }))
            }
        }

        // Create / Update operation
        (Some(payout_method), payout_token, Some(payout_data)) => {
            let lookup_key = vault::Vault::store_payout_method_data_in_locker(
                state,
                payout_token.to_owned(),
                payout_method,
                Some(customer_id.to_owned()),
                merchant_key_store,
            )
            .await?;

            // Update payout_token in payout_attempt table
            if payout_token.is_none() {
                let updated_payout_attempt = storage::PayoutAttemptUpdate::PayoutTokenUpdate {
                    payout_token: lookup_key,
                };
                payout_data.payout_attempt = db
                    .update_payout_attempt(
                        &payout_data.payout_attempt,
                        updated_payout_attempt,
                        &payout_data.payouts,
                        storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error updating token in payout attempt")?;
            }
            Ok(Some(payout_method.clone()))
        }

        // Ignore if nothing is passed
        _ => Ok(None),
    }
}

pub fn should_create_connector_transfer_method(
    payout_data: &PayoutData,
    connector_data: &api::ConnectorData,
) -> RouterResult<Option<String>> {
    let connector_transfer_method_id = payout_data.payment_method.as_ref().and_then(|pm| {
        let common_mandate_reference = pm
            .get_common_mandate_reference()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to deserialize connector mandate details")
            .ok()?;

        connector_data
            .merchant_connector_id
            .as_ref()
            .and_then(|merchant_connector_id| {
                common_mandate_reference
                    .payouts
                    .and_then(|payouts_mandate_reference| {
                        payouts_mandate_reference
                            .get(merchant_connector_id)
                            .and_then(|payouts_mandate_reference_record| {
                                payouts_mandate_reference_record.transfer_method_id.clone()
                            })
                    })
            })
    });

    Ok(connector_transfer_method_id)
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn save_payout_data_to_locker(
    state: &SessionState,
    payout_data: &mut PayoutData,
    customer_id: &id_type::CustomerId,
    payout_method_data: &api::PayoutMethodData,
    connector_mandate_details: Option<serde_json::Value>,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<()> {
    let payouts = &payout_data.payouts;
    let key_manager_state = state.into();
    let (mut locker_req, card_details, bank_details, wallet_details, payment_method_type) =
        match payout_method_data {
            payouts::PayoutMethodData::Card(card) => {
                let card_detail = api::CardDetail {
                    card_number: card.card_number.to_owned(),
                    card_holder_name: card.card_holder_name.to_owned(),
                    card_exp_month: card.expiry_month.to_owned(),
                    card_exp_year: card.expiry_year.to_owned(),
                    nick_name: None,
                    card_issuing_country: None,
                    card_network: None,
                    card_issuer: None,
                    card_type: None,
                };
                let payload = StoreLockerReq::LockerCard(StoreCardReq {
                    merchant_id: merchant_account.get_id().clone(),
                    merchant_customer_id: customer_id.to_owned(),
                    card: Card {
                        card_number: card.card_number.to_owned(),
                        name_on_card: card.card_holder_name.to_owned(),
                        card_exp_month: card.expiry_month.to_owned(),
                        card_exp_year: card.expiry_year.to_owned(),
                        card_brand: None,
                        card_isin: None,
                        nick_name: None,
                    },
                    requestor_card_reference: None,
                    ttl: state.conf.locker.ttl_for_storage_in_secs,
                });
                (
                    payload,
                    Some(card_detail),
                    None,
                    None,
                    api_enums::PaymentMethodType::Debit,
                )
            }
            _ => {
                let key = key_store.key.get_inner().peek();
                let key_manager_state: KeyManagerState = state.into();
                let enc_data = async {
                    serde_json::to_value(payout_method_data.to_owned())
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unable to encode payout method data")
                        .ok()
                        .map(|v| {
                            let secret: Secret<String> = Secret::new(v.to_string());
                            secret
                        })
                        .async_lift(|inner| async {
                            crypto_operation(
                                &key_manager_state,
                                type_name!(storage::PaymentMethod),
                                CryptoOperation::EncryptOptional(inner),
                                Identifier::Merchant(key_store.merchant_id.clone()),
                                key,
                            )
                            .await
                            .and_then(|val| val.try_into_optionaloperation())
                        })
                        .await
                }
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to encrypt payout method data")?
                .map(Encryption::from)
                .map(|e| e.into_inner())
                .map_or(Err(errors::ApiErrorResponse::InternalServerError), |e| {
                    Ok(hex::encode(e.peek()))
                })?;
                let payload = StoreLockerReq::LockerGeneric(StoreGenericReq {
                    merchant_id: merchant_account.get_id().to_owned(),
                    merchant_customer_id: customer_id.to_owned(),
                    enc_data,
                    ttl: state.conf.locker.ttl_for_storage_in_secs,
                });
                match payout_method_data {
                    payouts::PayoutMethodData::Bank(bank) => (
                        payload,
                        None,
                        Some(bank.to_owned()),
                        None,
                        api_enums::PaymentMethodType::foreign_from(bank),
                    ),
                    payouts::PayoutMethodData::Wallet(wallet) => (
                        payload,
                        None,
                        None,
                        Some(wallet.to_owned()),
                        api_enums::PaymentMethodType::foreign_from(wallet),
                    ),
                    payouts::PayoutMethodData::Card(_) => {
                        Err(errors::ApiErrorResponse::InternalServerError)?
                    }
                }
            }
        };

    // Store payout method in locker
    let stored_resp = cards::add_card_to_hs_locker(
        state,
        &locker_req,
        customer_id,
        api_enums::LockerChoice::HyperswitchCardVault,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let db = &*state.store;

    // Handle duplicates
    let (should_insert_in_pm_table, metadata_update) = match stored_resp.duplication_check {
        // Check if equivalent entry exists in payment_methods
        Some(duplication_check) => {
            let locker_ref = stored_resp.card_reference.clone();

            // Use locker ref as payment_method_id
            let existing_pm_by_pmid = db
                .find_payment_method(
                    &(state.into()),
                    key_store,
                    &locker_ref,
                    merchant_account.storage_scheme,
                )
                .await;

            match existing_pm_by_pmid {
                // If found, update locker's metadata [DELETE + INSERT OP], don't insert in payment_method's table
                Ok(pm) => (
                    false,
                    if duplication_check == DataDuplicationCheck::MetaDataChanged {
                        Some(pm.clone())
                    } else {
                        None
                    },
                ),

                // If not found, use locker ref as locker_id
                Err(err) => {
                    if err.current_context().is_db_not_found() {
                        match db
                            .find_payment_method_by_locker_id(
                                &(state.into()),
                                key_store,
                                &locker_ref,
                                merchant_account.storage_scheme,
                            )
                            .await
                        {
                            // If found, update locker's metadata [DELETE + INSERT OP], don't insert in payment_methods table
                            Ok(pm) => (
                                false,
                                if duplication_check == DataDuplicationCheck::MetaDataChanged {
                                    Some(pm.clone())
                                } else {
                                    None
                                },
                            ),
                            Err(err) => {
                                // If not found, update locker's metadata [DELETE + INSERT OP], and insert in payment_methods table
                                if err.current_context().is_db_not_found() {
                                    (true, None)

                                // Misc. DB errors
                                } else {
                                    Err(err)
                                        .change_context(
                                            errors::ApiErrorResponse::InternalServerError,
                                        )
                                        .attach_printable(
                                            "DB failures while finding payment method by locker ID",
                                        )?
                                }
                            }
                        }
                    // Misc. DB errors
                    } else {
                        Err(err)
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("DB failures while finding payment method by pm ID")?
                    }
                }
            }
        }

        // Not duplicate, should be inserted in payment_methods table
        None => (true, None),
    };

    // Form payment method entry and card's metadata whenever insertion or metadata update is required
    let (card_details_encrypted, new_payment_method) =
        if let (api::PayoutMethodData::Card(_), true, _)
        | (api::PayoutMethodData::Card(_), _, Some(_)) = (
            payout_method_data,
            should_insert_in_pm_table,
            metadata_update.as_ref(),
        ) {
            // Fetch card info from db
            let card_isin = card_details.as_ref().map(|c| c.card_number.get_card_isin());

            let mut payment_method = api::PaymentMethodCreate {
                payment_method: Some(api_enums::PaymentMethod::foreign_from(payout_method_data)),
                payment_method_type: Some(payment_method_type),
                payment_method_issuer: None,
                payment_method_issuer_code: None,
                bank_transfer: None,
                card: card_details.clone(),
                wallet: None,
                metadata: None,
                customer_id: Some(customer_id.to_owned()),
                card_network: None,
                client_secret: None,
                payment_method_data: None,
                billing: None,
                connector_mandate_details: None,
                network_transaction_id: None,
            };

            let pm_data = card_isin
                .clone()
                .async_and_then(|card_isin| async move {
                    db.get_card_info(&card_isin)
                        .await
                        .map_err(|error| services::logger::warn!(card_info_error=?error))
                        .ok()
                })
                .await
                .flatten()
                .map(|card_info| {
                    payment_method
                        .payment_method_issuer
                        .clone_from(&card_info.card_issuer);
                    payment_method.card_network =
                        card_info.card_network.clone().map(|cn| cn.to_string());
                    api::payment_methods::PaymentMethodsData::Card(
                        api::payment_methods::CardDetailsPaymentMethod {
                            last4_digits: card_details.as_ref().map(|c| c.card_number.get_last4()),
                            issuer_country: card_info.card_issuing_country,
                            expiry_month: card_details.as_ref().map(|c| c.card_exp_month.clone()),
                            expiry_year: card_details.as_ref().map(|c| c.card_exp_year.clone()),
                            nick_name: card_details.as_ref().and_then(|c| c.nick_name.clone()),
                            card_holder_name: card_details
                                .as_ref()
                                .and_then(|c| c.card_holder_name.clone()),

                            card_isin: card_isin.clone(),
                            card_issuer: card_info.card_issuer,
                            card_network: card_info.card_network,
                            card_type: card_info.card_type,
                            saved_to_locker: true,
                        },
                    )
                })
                .unwrap_or_else(|| {
                    api::payment_methods::PaymentMethodsData::Card(
                        api::payment_methods::CardDetailsPaymentMethod {
                            last4_digits: card_details.as_ref().map(|c| c.card_number.get_last4()),
                            issuer_country: None,
                            expiry_month: card_details.as_ref().map(|c| c.card_exp_month.clone()),
                            expiry_year: card_details.as_ref().map(|c| c.card_exp_year.clone()),
                            nick_name: card_details.as_ref().and_then(|c| c.nick_name.clone()),
                            card_holder_name: card_details
                                .as_ref()
                                .and_then(|c| c.card_holder_name.clone()),

                            card_isin: card_isin.clone(),
                            card_issuer: None,
                            card_network: None,
                            card_type: None,
                            saved_to_locker: true,
                        },
                    )
                });
            (
                Some(
                    cards::create_encrypted_data(&key_manager_state, key_store, pm_data)
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unable to encrypt customer details")?,
                ),
                payment_method,
            )
        } else {
            (
                None,
                api::PaymentMethodCreate {
                    payment_method: Some(api_enums::PaymentMethod::foreign_from(
                        payout_method_data,
                    )),
                    payment_method_type: Some(payment_method_type),
                    payment_method_issuer: None,
                    payment_method_issuer_code: None,
                    bank_transfer: bank_details,
                    card: None,
                    wallet: wallet_details,
                    metadata: None,
                    customer_id: Some(customer_id.to_owned()),
                    card_network: None,
                    client_secret: None,
                    payment_method_data: None,
                    billing: None,
                    connector_mandate_details: None,
                    network_transaction_id: None,
                },
            )
        };

    // Insert new entry in payment_methods table
    if should_insert_in_pm_table {
        let payment_method_id = common_utils::generate_id(consts::ID_LENGTH, "pm");
        payout_data.payment_method = Some(
            cards::create_payment_method(
                state,
                &new_payment_method,
                customer_id,
                &payment_method_id,
                Some(stored_resp.card_reference.clone()),
                merchant_account.get_id(),
                None,
                None,
                card_details_encrypted.clone().map(Into::into),
                key_store,
                connector_mandate_details,
                None,
                None,
                merchant_account.storage_scheme,
                None,
                None,
                None,
                None,
                None,
            )
            .await?,
        );
    }

    /*  1. Delete from locker
     *  2. Create new entry in locker
     *  3. Handle creation response from locker
     *  4. Update card's metadata in payment_methods table
     */
    if let Some(existing_pm) = metadata_update {
        let card_reference = &existing_pm
            .locker_id
            .clone()
            .unwrap_or(existing_pm.payment_method_id.clone());
        // Delete from locker
        cards::delete_card_from_hs_locker(
            state,
            customer_id,
            merchant_account.get_id(),
            card_reference,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to delete PMD from locker as a part of metadata update operation",
        )?;

        locker_req.update_requestor_card_reference(Some(card_reference.to_string()));

        // Store in locker
        let stored_resp = cards::add_card_to_hs_locker(
            state,
            &locker_req,
            customer_id,
            api_enums::LockerChoice::HyperswitchCardVault,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError);

        // Check if locker operation was successful or not, if not, delete the entry from payment_methods table
        if let Err(err) = stored_resp {
            logger::error!(vault_err=?err);
            db.delete_payment_method_by_merchant_id_payment_method_id(
                &(state.into()),
                key_store,
                merchant_account.get_id(),
                &existing_pm.payment_method_id,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

            Err(errors::ApiErrorResponse::InternalServerError).attach_printable(
                "Failed to insert PMD from locker as a part of metadata update operation",
            )?
        };

        // Update card's metadata in payment_methods table
        let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
            payment_method_data: card_details_encrypted.map(Into::into),
        };
        payout_data.payment_method = Some(
            db.update_payment_method(
                &(state.into()),
                key_store,
                existing_pm,
                pm_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add payment method in db")?,
        );
    };

    // Store card_reference in payouts table
    let payout_method_id = match &payout_data.payment_method {
        Some(pm) => pm.payment_method_id.clone(),
        None => stored_resp.card_reference.to_owned(),
    };

    let updated_payout = storage::PayoutsUpdate::PayoutMethodIdUpdate { payout_method_id };

    payout_data.payouts = db
        .update_payout(
            payouts,
            updated_payout,
            &payout_data.payout_attempt,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payouts in saved payout method")?;

    Ok(())
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn save_payout_data_to_locker(
    _state: &SessionState,
    _payout_data: &mut PayoutData,
    _customer_id: &id_type::CustomerId,
    _payout_method_data: &api::PayoutMethodData,
    _connector_mandate_details: Option<serde_json::Value>,
    _merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
) -> RouterResult<()> {
    todo!()
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub(super) async fn get_or_create_customer_details(
    _state: &SessionState,
    _customer_details: &CustomerDetails,
    _merchant_account: &domain::MerchantAccount,
    _key_store: &domain::MerchantKeyStore,
) -> RouterResult<Option<domain::Customer>> {
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub(super) async fn get_or_create_customer_details(
    state: &SessionState,
    customer_details: &CustomerDetails,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> RouterResult<Option<domain::Customer>> {
    let db: &dyn StorageInterface = &*state.store;
    // Create customer_id if not passed in request
    let customer_id = customer_details
        .customer_id
        .clone()
        .unwrap_or_else(generate_customer_id_of_default_length);

    let merchant_id = merchant_account.get_id();
    let key = key_store.key.get_inner().peek();
    let key_manager_state = &state.into();

    match db
        .find_customer_optional_by_customer_id_merchant_id(
            key_manager_state,
            &customer_id,
            merchant_id,
            key_store,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?
    {
        // Customer found
        Some(customer) => Ok(Some(customer)),

        // Customer not found
        // create only if atleast one of the fields were provided for customer creation or else throw error
        None => {
            if customer_details.name.is_some()
                || customer_details.email.is_some()
                || customer_details.phone.is_some()
                || customer_details.phone_country_code.is_some()
            {
                let encrypted_data = crypto_operation(
                    &state.into(),
                    type_name!(domain::Customer),
                    CryptoOperation::BatchEncrypt(
                        domain::FromRequestEncryptableCustomer::to_encryptable(
                            domain::FromRequestEncryptableCustomer {
                                name: customer_details.name.clone(),
                                email: customer_details
                                    .email
                                    .clone()
                                    .map(|a| a.expose().switch_strategy()),
                                phone: customer_details.phone.clone(),
                            },
                        ),
                    ),
                    Identifier::Merchant(key_store.merchant_id.clone()),
                    key,
                )
                .await
                .and_then(|val| val.try_into_batchoperation())
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to encrypt customer")?;
                let encryptable_customer =
                    domain::FromRequestEncryptableCustomer::from_encryptable(encrypted_data)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to form EncryptableCustomer")?;

                let customer = domain::Customer {
                    customer_id: customer_id.clone(),
                    merchant_id: merchant_id.to_owned().clone(),
                    name: encryptable_customer.name,
                    email: encryptable_customer.email.map(|email| {
                        let encryptable: Encryptable<Secret<String, pii::EmailStrategy>> =
                            Encryptable::new(
                                email.clone().into_inner().switch_strategy(),
                                email.into_encrypted(),
                            );
                        encryptable
                    }),
                    phone: encryptable_customer.phone,
                    description: None,
                    phone_country_code: customer_details.phone_country_code.to_owned(),
                    metadata: None,
                    connector_customer: None,
                    created_at: common_utils::date_time::now(),
                    modified_at: common_utils::date_time::now(),
                    address_id: None,
                    default_payment_method_id: None,
                    updated_by: None,
                    version: hyperswitch_domain_models::consts::API_VERSION,
                };

                Ok(Some(
                    db.insert_customer(
                        customer,
                        key_manager_state,
                        key_store,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable_lazy(|| {
                        format!(
                            "Failed to insert customer [id - {:?}] for merchant [id - {:?}]",
                            customer_id, merchant_id
                        )
                    })?,
                ))
            } else {
                Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!("customer for id - {:?} not found", customer_id),
                }))
            }
        }
    }
}

#[cfg(all(feature = "payouts", feature = "v1"))]
pub async fn decide_payout_connector(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    request_straight_through: Option<api::routing::StraightThroughAlgorithm>,
    routing_data: &mut storage::RoutingData,
    payout_data: &mut PayoutData,
    eligible_connectors: Option<Vec<enums::RoutableConnectors>>,
) -> RouterResult<api::ConnectorCallType> {
    // 1. For existing attempts, use stored connector
    let payout_attempt = &payout_data.payout_attempt;
    if let Some(connector_name) = payout_attempt.connector.clone() {
        // Connector was already decided previously, use the same connector
        let connector_data = api::ConnectorData::get_payout_connector_by_name(
            &state.conf.connectors,
            &connector_name,
            api::GetToken::Connector,
            payout_attempt.merchant_connector_id.clone(),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Invalid connector name received in 'routed_through'")?;

        routing_data.routed_through = Some(connector_name.clone());
        return Ok(api::ConnectorCallType::PreDetermined(connector_data));
    }

    // Validate and get the business_profile from payout_attempt
    let business_profile = core_utils::validate_and_get_business_profile(
        state.store.as_ref(),
        &(state).into(),
        key_store,
        Some(&payout_attempt.profile_id),
        merchant_account.get_id(),
    )
    .await?
    .get_required_value("Profile")?;

    // 2. Check routing algorithm passed in the request
    if let Some(routing_algorithm) = request_straight_through {
        let (mut connectors, check_eligibility) =
            routing::perform_straight_through_routing(&routing_algorithm, None)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed execution of straight through routing")?;

        if check_eligibility {
            connectors = routing::perform_eligibility_analysis_with_fallback(
                state,
                key_store,
                connectors,
                &TransactionData::Payout(payout_data),
                eligible_connectors,
                &business_profile,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed eligibility analysis and fallback")?;
        }

        let first_connector_choice = connectors
            .first()
            .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
            .attach_printable("Empty connector list returned")?
            .clone();

        let connector_data = connectors
            .into_iter()
            .map(|conn| {
                api::ConnectorData::get_payout_connector_by_name(
                    &state.conf.connectors,
                    &conn.connector.to_string(),
                    api::GetToken::Connector,
                    payout_attempt.merchant_connector_id.clone(),
                )
            })
            .collect::<CustomResult<Vec<_>, _>>()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received")?;

        routing_data.routed_through = Some(first_connector_choice.connector.to_string());
        routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;

        routing_data.routing_info.algorithm = Some(routing_algorithm);
        return Ok(api::ConnectorCallType::Retryable(connector_data));
    }

    // 3. Check algorithm passed in routing data
    if let Some(ref routing_algorithm) = routing_data.algorithm {
        let (mut connectors, check_eligibility) =
            routing::perform_straight_through_routing(routing_algorithm, None)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed execution of straight through routing")?;

        if check_eligibility {
            connectors = routing::perform_eligibility_analysis_with_fallback(
                state,
                key_store,
                connectors,
                &TransactionData::Payout(payout_data),
                eligible_connectors,
                &business_profile,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed eligibility analysis and fallback")?;
        }

        let first_connector_choice = connectors
            .first()
            .ok_or(errors::ApiErrorResponse::IncorrectPaymentMethodConfiguration)
            .attach_printable("Empty connector list returned")?
            .clone();

        connectors.remove(0);

        let connector_data = connectors
            .into_iter()
            .map(|conn| {
                api::ConnectorData::get_payout_connector_by_name(
                    &state.conf.connectors,
                    &conn.connector.to_string(),
                    api::GetToken::Connector,
                    payout_attempt.merchant_connector_id.clone(),
                )
            })
            .collect::<CustomResult<Vec<_>, _>>()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid connector name received")?;

        routing_data.routed_through = Some(first_connector_choice.connector.to_string());
        routing_data.merchant_connector_id = first_connector_choice.merchant_connector_id;

        return Ok(api::ConnectorCallType::Retryable(connector_data));
    }

    // 4. Route connector
    route_connector_v1_for_payouts(
        state,
        merchant_account,
        &payout_data.business_profile,
        key_store,
        payout_data,
        routing_data,
        eligible_connectors,
    )
    .await
}

pub async fn get_default_payout_connector(
    _state: &SessionState,
    request_connector: Option<serde_json::Value>,
) -> CustomResult<api::ConnectorChoice, errors::ApiErrorResponse> {
    Ok(request_connector.map_or(
        api::ConnectorChoice::Decide,
        api::ConnectorChoice::StraightThrough,
    ))
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub fn should_call_payout_connector_create_customer<'a>(
    state: &'a SessionState,
    connector: &'a api::ConnectorData,
    customer: &'a Option<domain::Customer>,
    connector_label: &'a str,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    match enums::PayoutConnectors::try_from(connector.connector_name) {
        Ok(connector) => {
            let connector_needs_customer = state
                .conf
                .connector_customer
                .payout_connector_list
                .contains(&connector);

            if connector_needs_customer {
                let connector_customer_details = customer
                    .as_ref()
                    .and_then(|customer| customer.get_connector_customer_id(connector_label));
                let should_call_connector = connector_customer_details.is_none();
                (should_call_connector, connector_customer_details)
            } else {
                (false, None)
            }
        }
        _ => (false, None),
    }
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub fn should_call_payout_connector_create_customer<'a>(
    state: &'a SessionState,
    connector: &'a api::ConnectorData,
    customer: &'a Option<domain::Customer>,
    merchant_connector_id: &'a id_type::MerchantConnectorAccountId,
) -> (bool, Option<&'a str>) {
    // Check if create customer is required for the connector
    match enums::PayoutConnectors::try_from(connector.connector_name) {
        Ok(connector) => {
            let connector_needs_customer = state
                .conf
                .connector_customer
                .payout_connector_list
                .contains(&connector);

            if connector_needs_customer {
                let connector_customer_details = customer
                    .as_ref()
                    .and_then(|customer| customer.get_connector_customer_id(merchant_connector_id));
                let should_call_connector = connector_customer_details.is_none();
                (should_call_connector, connector_customer_details)
            } else {
                (false, None)
            }
        }
        _ => (false, None),
    }
}

pub async fn get_gsm_record(
    state: &SessionState,
    error_code: Option<String>,
    error_message: Option<String>,
    connector_name: Option<String>,
    flow: &str,
) -> Option<storage::gsm::GatewayStatusMap> {
    let connector_name = connector_name.unwrap_or_default();
    let get_gsm = || async {
        state.store.find_gsm_rule(
                connector_name.clone(),
                flow.to_string(),
                "sub_flow".to_string(),
                error_code.clone().unwrap_or_default(), // TODO: make changes in connector to get a mandatory code in case of success or error response
                error_message.clone().unwrap_or_default(),
            )
            .await
            .map_err(|err| {
                if err.current_context().is_db_not_found() {
                    logger::warn!(
                        "GSM miss for connector - {}, flow - {}, error_code - {:?}, error_message - {:?}",
                        connector_name,
                        flow,
                        error_code,
                        error_message
                    );
                    metrics::AUTO_PAYOUT_RETRY_GSM_MISS_COUNT.add( 1, &[]);
                } else {
                    metrics::AUTO_PAYOUT_RETRY_GSM_FETCH_FAILURE_COUNT.add( 1, &[]);
                };
                err.change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("failed to fetch decision from gsm")
            })
    };
    get_gsm()
        .await
        .inspect_err(|err| {
            // warn log should suffice here because we are not propagating this error
            logger::warn!(get_gsm_decision_fetch_error=?err, "error fetching gsm decision");
        })
        .ok()
}

pub fn is_payout_initiated(status: api_enums::PayoutStatus) -> bool {
    !matches!(
        status,
        api_enums::PayoutStatus::RequiresCreation
            | api_enums::PayoutStatus::RequiresConfirmation
            | api_enums::PayoutStatus::RequiresPayoutMethodData
            | api_enums::PayoutStatus::RequiresVendorAccountCreation
            | api_enums::PayoutStatus::Initiated
    )
}

pub(crate) fn validate_payout_status_against_not_allowed_statuses(
    payout_status: api_enums::PayoutStatus,
    not_allowed_statuses: &[api_enums::PayoutStatus],
    action: &'static str,
) -> Result<(), errors::ApiErrorResponse> {
    fp_utils::when(not_allowed_statuses.contains(&payout_status), || {
        Err(errors::ApiErrorResponse::PreconditionFailed {
            message: format!(
                "You cannot {action} this payout because it has status {payout_status}",
            ),
        })
    })
}

pub fn is_payout_terminal_state(status: api_enums::PayoutStatus) -> bool {
    !matches!(
        status,
        api_enums::PayoutStatus::RequiresCreation
            | api_enums::PayoutStatus::RequiresConfirmation
            | api_enums::PayoutStatus::RequiresPayoutMethodData
            | api_enums::PayoutStatus::RequiresVendorAccountCreation
            // Initiated by the underlying connector
            | api_enums::PayoutStatus::Pending
            | api_enums::PayoutStatus::Initiated
            | api_enums::PayoutStatus::RequiresFulfillment
    )
}

pub fn should_call_retrieve(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::Pending | api_enums::PayoutStatus::Initiated
    )
}

pub fn is_payout_err_state(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::Cancelled
            | api_enums::PayoutStatus::Failed
            | api_enums::PayoutStatus::Ineligible
    )
}

pub fn is_eligible_for_local_payout_cancellation(status: api_enums::PayoutStatus) -> bool {
    matches!(
        status,
        api_enums::PayoutStatus::RequiresCreation
            | api_enums::PayoutStatus::RequiresConfirmation
            | api_enums::PayoutStatus::RequiresPayoutMethodData
            | api_enums::PayoutStatus::RequiresVendorAccountCreation
    )
}

#[cfg(feature = "olap")]
pub(super) async fn filter_by_constraints(
    db: &dyn StorageInterface,
    constraints: &api::PayoutListConstraints,
    merchant_id: &id_type::MerchantId,
    storage_scheme: storage::enums::MerchantStorageScheme,
) -> CustomResult<Vec<storage::Payouts>, errors::DataStorageError> {
    let result = db
        .filter_payouts_by_constraints(merchant_id, &constraints.clone().into(), storage_scheme)
        .await?;
    Ok(result)
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub async fn update_payouts_and_payout_attempt(
    _payout_data: &mut PayoutData,
    _merchant_account: &domain::MerchantAccount,
    _req: &payouts::PayoutCreateRequest,
    _state: &SessionState,
    _merchant_key_store: &domain::MerchantKeyStore,
) -> CustomResult<(), errors::ApiErrorResponse> {
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub async fn update_payouts_and_payout_attempt(
    payout_data: &mut PayoutData,
    merchant_account: &domain::MerchantAccount,
    req: &payouts::PayoutCreateRequest,
    state: &SessionState,
    merchant_key_store: &domain::MerchantKeyStore,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let payout_attempt = payout_data.payout_attempt.to_owned();
    let status = payout_attempt.status;
    let payout_id = payout_attempt.payout_id.clone();
    // Verify update feasibility
    if is_payout_terminal_state(status) || is_payout_initiated(status) {
        return Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: format!(
                "Payout {} cannot be updated for status {}",
                payout_id, status
            ),
        }));
    }

    // Fetch customer details from request and create new or else use existing customer that was attached
    let customer = get_customer_details_from_request(req);
    let customer_id = if customer.customer_id.is_some()
        || customer.name.is_some()
        || customer.email.is_some()
        || customer.phone.is_some()
        || customer.phone_country_code.is_some()
    {
        payout_data.customer_details =
            get_or_create_customer_details(state, &customer, merchant_account, merchant_key_store)
                .await?;
        payout_data
            .customer_details
            .as_ref()
            .map(|customer| customer.customer_id.clone())
    } else {
        payout_data.payouts.customer_id.clone()
    };

    // We have to do this because the function that is being used to create / get address is from payments
    // which expects a payment_id
    let payout_id_as_payment_id_type =
        id_type::PaymentId::try_from(std::borrow::Cow::Owned(payout_id.clone()))
            .change_context(errors::ApiErrorResponse::InvalidRequestData {
                message: "payout_id contains invalid data".to_string(),
            })
            .attach_printable("Error converting payout_id to PaymentId type")?;

    // Fetch address details from request and create new or else use existing address that was attached
    let billing_address = payment_helpers::create_or_find_address_for_payment_by_request(
        state,
        req.billing.as_ref(),
        None,
        merchant_account.get_id(),
        customer_id.as_ref(),
        merchant_key_store,
        &payout_id_as_payment_id_type,
        merchant_account.storage_scheme,
    )
    .await?;
    let address_id = if billing_address.is_some() {
        payout_data.billing_address = billing_address;
        payout_data
            .billing_address
            .as_ref()
            .map(|address| address.address_id.clone())
    } else {
        payout_data.payouts.address_id.clone()
    };

    // Update DB with new data
    let payouts = payout_data.payouts.to_owned();
    let amount = MinorUnit::from(req.amount.unwrap_or(payouts.amount.into()));
    let updated_payouts = storage::PayoutsUpdate::Update {
        amount,
        destination_currency: req
            .currency
            .to_owned()
            .unwrap_or(payouts.destination_currency),
        source_currency: req.currency.to_owned().unwrap_or(payouts.source_currency),
        description: req
            .description
            .to_owned()
            .clone()
            .or(payouts.description.clone()),
        recurring: req.recurring.to_owned().unwrap_or(payouts.recurring),
        auto_fulfill: req.auto_fulfill.to_owned().unwrap_or(payouts.auto_fulfill),
        return_url: req
            .return_url
            .to_owned()
            .clone()
            .or(payouts.return_url.clone()),
        entity_type: req.entity_type.to_owned().unwrap_or(payouts.entity_type),
        metadata: req.metadata.clone().or(payouts.metadata.clone()),
        status: Some(status),
        profile_id: Some(payout_attempt.profile_id.clone()),
        confirm: req.confirm.to_owned(),
        payout_type: req
            .payout_type
            .to_owned()
            .or(payouts.payout_type.to_owned()),
        address_id: address_id.clone(),
        customer_id: customer_id.clone(),
    };
    let db = &*state.store;
    payout_data.payouts = db
        .update_payout(
            &payouts,
            updated_payouts,
            &payout_attempt,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error updating payouts")?;
    let updated_business_country =
        payout_attempt
            .business_country
            .map_or(req.business_country.to_owned(), |c| {
                req.business_country
                    .to_owned()
                    .and_then(|nc| if nc != c { Some(nc) } else { None })
            });
    let updated_business_label =
        payout_attempt
            .business_label
            .map_or(req.business_label.to_owned(), |l| {
                req.business_label
                    .to_owned()
                    .and_then(|nl| if nl != l { Some(nl) } else { None })
            });
    if updated_business_country.is_some()
        || updated_business_label.is_some()
        || customer_id.is_some()
        || address_id.is_some()
    {
        let payout_attempt = &payout_data.payout_attempt;
        let updated_payout_attempt = storage::PayoutAttemptUpdate::BusinessUpdate {
            business_country: updated_business_country,
            business_label: updated_business_label,
            address_id,
            customer_id,
        };
        payout_data.payout_attempt = db
            .update_payout_attempt(
                payout_attempt,
                updated_payout_attempt,
                &payout_data.payouts,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating payout_attempt")?;
    }
    Ok(())
}

pub(super) fn get_customer_details_from_request(
    request: &payouts::PayoutCreateRequest,
) -> CustomerDetails {
    let customer_id = request.get_customer_id().map(ToOwned::to_owned);

    let customer_name = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.name.clone())
        .or(request.name.clone());

    let customer_email = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.email.clone())
        .or(request.email.clone());

    let customer_phone = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone.clone())
        .or(request.phone.clone());

    let customer_phone_code = request
        .customer
        .as_ref()
        .and_then(|customer_details| customer_details.phone_country_code.clone())
        .or(request.phone_country_code.clone());

    CustomerDetails {
        customer_id,
        name: customer_name,
        email: customer_email,
        phone: customer_phone,
        phone_country_code: customer_phone_code,
    }
}

pub async fn get_translated_unified_code_and_message(
    state: &SessionState,
    unified_code: Option<&UnifiedCode>,
    unified_message: Option<&UnifiedMessage>,
    locale: &str,
) -> CustomResult<Option<UnifiedMessage>, errors::ApiErrorResponse> {
    Ok(unified_code
        .zip(unified_message)
        .async_and_then(|(code, message)| async {
            payment_helpers::get_unified_translation(
                state,
                code.0.clone(),
                message.0.clone(),
                locale.to_string(),
            )
            .await
            .map(UnifiedMessage::try_from)
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "unified_message",
        })?
        .or_else(|| unified_message.cloned()))
}

pub async fn get_additional_payout_data(
    pm_data: &api::PayoutMethodData,
    db: &dyn StorageInterface,
    profile_id: &id_type::ProfileId,
) -> Option<payout_additional::AdditionalPayoutMethodData> {
    match pm_data {
        api::PayoutMethodData::Card(card_data) => {
            let card_isin = Some(card_data.card_number.get_card_isin());
            let enable_extended_bin =db
            .find_config_by_key_unwrap_or(
                format!("{}_enable_extended_card_bin", profile_id.get_string_repr()).as_str(),
             Some("false".to_string()))
            .await.map_err(|err| services::logger::error!(message="Failed to fetch the config", extended_card_bin_error=?err)).ok();

            let card_extended_bin = match enable_extended_bin {
                Some(config) if config.config == "true" => {
                    Some(card_data.card_number.get_extended_card_bin())
                }
                _ => None,
            };
            let last4 = Some(card_data.card_number.get_last4());

            let card_info = card_isin
                .clone()
                .async_and_then(|card_isin| async move {
                    db.get_card_info(&card_isin)
                        .await
                        .map_err(|error| services::logger::warn!(card_info_error=?error))
                        .ok()
                })
                .await
                .flatten()
                .map(|card_info| {
                    payout_additional::AdditionalPayoutMethodData::Card(Box::new(
                        payout_additional::CardAdditionalData {
                            card_issuer: card_info.card_issuer,
                            card_network: card_info.card_network.clone(),
                            bank_code: card_info.bank_code,
                            card_type: card_info.card_type,
                            card_issuing_country: card_info.card_issuing_country,
                            last4: last4.clone(),
                            card_isin: card_isin.clone(),
                            card_extended_bin: card_extended_bin.clone(),
                            card_exp_month: Some(card_data.expiry_month.clone()),
                            card_exp_year: Some(card_data.expiry_year.clone()),
                            card_holder_name: card_data.card_holder_name.clone(),
                        },
                    ))
                });
            Some(card_info.unwrap_or_else(|| {
                payout_additional::AdditionalPayoutMethodData::Card(Box::new(
                    payout_additional::CardAdditionalData {
                        card_issuer: None,
                        card_network: None,
                        bank_code: None,
                        card_type: None,
                        card_issuing_country: None,
                        last4,
                        card_isin,
                        card_extended_bin,
                        card_exp_month: Some(card_data.expiry_month.clone()),
                        card_exp_year: Some(card_data.expiry_year.clone()),
                        card_holder_name: card_data.card_holder_name.clone(),
                    },
                ))
            }))
        }
        api::PayoutMethodData::Bank(bank_data) => {
            Some(payout_additional::AdditionalPayoutMethodData::Bank(
                Box::new(bank_data.to_owned().into()),
            ))
        }
        api::PayoutMethodData::Wallet(wallet_data) => {
            Some(payout_additional::AdditionalPayoutMethodData::Wallet(
                Box::new(wallet_data.to_owned().into()),
            ))
        }
    }
}
