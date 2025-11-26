use std::collections::HashMap;

use ::payment_methods::controller::PaymentMethodsController;
#[cfg(feature = "v1")]
use api_models::payment_methods::PaymentMethodsData;
use api_models::{
    payment_methods::PaymentMethodDataWalletInfo, payments::ConnectorMandateReferenceId,
};
use common_enums::{ConnectorMandateStatus, PaymentMethod};
use common_types::callback_mapper::CallbackMapperData;
use common_utils::{
    crypto::Encryptable,
    ext_traits::{AsyncExt, Encode, ValueExt},
    id_type,
    metrics::utils::record_operation_time,
    pii,
};
use diesel_models::business_profile::ExternalVaultConnectorDetails;
use error_stack::{report, ResultExt};
#[cfg(feature = "v1")]
use hyperswitch_domain_models::{
    callback_mapper::CallbackMapper,
    mandates::{CommonMandateReference, PaymentsMandateReference, PaymentsMandateReferenceRecord},
    payment_method_data,
};
use masking::{ExposeInterface, Secret};
use router_env::{instrument, tracing};

use super::helpers;
#[cfg(feature = "v1")]
use crate::core::payment_methods::{
    get_payment_method_custom_data, vault_payment_method_external_v1,
};
use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        mandate,
        payment_methods::{
            self,
            cards::{create_encrypted_data, PmCards},
            network_tokenization,
        },
        payments,
    },
    logger,
    routes::{metrics, SessionState},
    services,
    types::{
        self,
        api::{self, CardDetailFromLocker, CardDetailsPaymentMethod, PaymentMethodCreateExt},
        domain, payment_methods as pm_types,
        storage::enums as storage_enums,
    },
    utils::{generate_id, OptionExt},
};

#[cfg(feature = "v1")]
async fn save_in_locker(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method_request: api::PaymentMethodCreate,
    card_detail: Option<api::CardDetail>,
    business_profile: &domain::Profile,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    match &business_profile.external_vault_details {
        domain::ExternalVaultDetails::ExternalVaultEnabled(external_vault_details) => {
            logger::info!("External vault is enabled, using vault_payment_method_external_v1");

            Box::pin(save_in_locker_external(
                state,
                platform,
                payment_method_request,
                card_detail,
                external_vault_details,
            ))
            .await
        }
        domain::ExternalVaultDetails::Skip => {
            // Use internal vault (locker)
            save_in_locker_internal(state, platform, payment_method_request, card_detail).await
        }
    }
}

pub struct SavePaymentMethodData<Req> {
    request: Req,
    response: Result<types::PaymentsResponseData, types::ErrorResponse>,
    payment_method_token: Option<types::PaymentMethodToken>,
    payment_method: PaymentMethod,
    attempt_status: common_enums::AttemptStatus,
}

impl<F, Req: Clone> From<&types::RouterData<F, Req, types::PaymentsResponseData>>
    for SavePaymentMethodData<Req>
{
    fn from(router_data: &types::RouterData<F, Req, types::PaymentsResponseData>) -> Self {
        Self {
            request: router_data.request.clone(),
            response: router_data.response.clone(),
            payment_method_token: router_data.payment_method_token.clone(),
            payment_method: router_data.payment_method,
            attempt_status: router_data.status,
        }
    }
}
pub struct SavePaymentMethodDataResponse {
    pub payment_method_id: Option<String>,
    pub payment_method_status: Option<common_enums::PaymentMethodStatus>,
    pub connector_mandate_reference_id: Option<ConnectorMandateReferenceId>,
}
#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn save_payment_method<FData>(
    state: &SessionState,
    connector_name: String,
    save_payment_method_data: SavePaymentMethodData<FData>,
    customer_id: Option<id_type::CustomerId>,
    platform: &domain::Platform,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    billing_name: Option<Secret<String>>,
    payment_method_billing_address: Option<&hyperswitch_domain_models::address::Address>,
    business_profile: &domain::Profile,
    mut original_connector_mandate_reference_id: Option<ConnectorMandateReferenceId>,
    merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    vault_operation: Option<hyperswitch_domain_models::payments::VaultOperation>,
    payment_method_info: Option<domain::PaymentMethod>,
) -> RouterResult<SavePaymentMethodDataResponse>
where
    FData: mandate::MandateBehaviour + Clone,
{
    let mut pm_status = None;
    let cards = PmCards { state, platform };
    match save_payment_method_data.response {
        Ok(responses) => {
            let db = &*state.store;
            let token_store = state
                .conf
                .tokenization
                .0
                .get(&connector_name.to_string())
                .map(|token_filter| token_filter.long_lived_token)
                .unwrap_or(false);

            let network_transaction_id = match &responses {
                types::PaymentsResponseData::TransactionResponse { network_txn_id, .. } => {
                    network_txn_id.clone()
                }
                _ => None,
            };

            let network_transaction_id =
                if save_payment_method_data.request.get_setup_future_usage()
                    == Some(storage_enums::FutureUsage::OffSession)
                {
                    if network_transaction_id.is_some() {
                        network_transaction_id
                    } else {
                        logger::info!("Skip storing network transaction id");
                        None
                    }
                } else {
                    None
                };

            let connector_token = if token_store {
                let tokens = save_payment_method_data
                    .payment_method_token
                    .to_owned()
                    .get_required_value("payment_token")?;
                let token = match tokens {
                    types::PaymentMethodToken::Token(connector_token) => connector_token.expose(),
                    types::PaymentMethodToken::ApplePayDecrypt(_) => {
                        Err(errors::ApiErrorResponse::NotSupported {
                            message: "Apple Pay Decrypt token is not supported".to_string(),
                        })?
                    }
                    types::PaymentMethodToken::PazeDecrypt(_) => {
                        Err(errors::ApiErrorResponse::NotSupported {
                            message: "Paze Decrypt token is not supported".to_string(),
                        })?
                    }
                    types::PaymentMethodToken::GooglePayDecrypt(_) => {
                        Err(errors::ApiErrorResponse::NotSupported {
                            message: "Google Pay Decrypt token is not supported".to_string(),
                        })?
                    }
                };
                Some((connector_name, token))
            } else {
                None
            };

            let mandate_data_customer_acceptance = save_payment_method_data
                .request
                .get_setup_mandate_details()
                .and_then(|mandate_data| mandate_data.customer_acceptance.clone());

            let customer_acceptance = save_payment_method_data
                .request
                .get_customer_acceptance()
                .or(mandate_data_customer_acceptance.clone())
                .map(|ca| ca.encode_to_value())
                .transpose()
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to serialize customer acceptance to value")?;

            let (connector_mandate_id, mandate_metadata, connector_mandate_request_reference_id) =
                match responses {
                    types::PaymentsResponseData::TransactionResponse {
                        mandate_reference, ..
                    } => {
                        if let Some(ref mandate_ref) = *mandate_reference {
                            (
                                mandate_ref.connector_mandate_id.clone(),
                                mandate_ref.mandate_metadata.clone(),
                                mandate_ref.connector_mandate_request_reference_id.clone(),
                            )
                        } else {
                            (None, None, None)
                        }
                    }
                    _ => (None, None, None),
                };

            let pm_id = if customer_acceptance.is_some() {
                let payment_method_data =
                    save_payment_method_data.request.get_payment_method_data();
                let payment_method_create_request =
                    payment_methods::get_payment_method_create_request(
                        Some(&payment_method_data),
                        Some(save_payment_method_data.payment_method),
                        payment_method_type,
                        &customer_id.clone(),
                        billing_name,
                        payment_method_billing_address,
                    )
                    .await?;
                let payment_methods_data =
                    &save_payment_method_data.request.get_payment_method_data();

                let co_badged_card_data = payment_methods_data.get_co_badged_card_data();

                let customer_id = customer_id.to_owned().get_required_value("customer_id")?;
                let merchant_id = platform.get_processor().get_account().get_id();
                let is_network_tokenization_enabled =
                    business_profile.is_network_tokenization_enabled;
                let (
                    (mut resp, duplication_check, network_token_requestor_ref_id),
                    network_token_resp,
                ) = if !state.conf.locker.locker_enabled {
                    let (res, dc) = skip_saving_card_in_locker(
                        platform,
                        payment_method_create_request.to_owned(),
                    )
                    .await?;
                    ((res, dc, None), None)
                } else {
                    let payment_method_status = common_enums::PaymentMethodStatus::from(
                        save_payment_method_data.attempt_status,
                    );
                    pm_status = Some(payment_method_status);
                    save_card_and_network_token_in_locker(
                        state,
                        customer_id.clone(),
                        payment_method_status,
                        payment_method_data.clone(),
                        vault_operation,
                        payment_method_info,
                        platform,
                        payment_method_create_request.clone(),
                        is_network_tokenization_enabled,
                        business_profile,
                    )
                    .await?
                };
                let network_token_locker_id = match network_token_resp {
                    Some(ref token_resp) => {
                        if network_token_requestor_ref_id.is_some() {
                            Some(token_resp.payment_method_id.clone())
                        } else {
                            None
                        }
                    }
                    None => None,
                };

                let optional_pm_details = match (resp.card.as_ref(), payment_method_data) {
                    (Some(card), _) => Some(PaymentMethodsData::Card(
                        CardDetailsPaymentMethod::from((card.clone(), co_badged_card_data)),
                    )),
                    (
                        _,
                        domain::PaymentMethodData::Wallet(domain::WalletData::ApplePay(applepay)),
                    ) => Some(PaymentMethodsData::WalletDetails(
                        PaymentMethodDataWalletInfo::from(applepay),
                    )),
                    (
                        _,
                        domain::PaymentMethodData::Wallet(domain::WalletData::GooglePay(googlepay)),
                    ) => Some(PaymentMethodsData::WalletDetails(
                        PaymentMethodDataWalletInfo::from(googlepay),
                    )),
                    _ => None,
                };

                let key_manager_state = state.into();
                let pm_data_encrypted: Option<Encryptable<Secret<serde_json::Value>>> =
                    optional_pm_details
                        .async_map(|pm| {
                            create_encrypted_data(
                                &key_manager_state,
                                platform.get_processor().get_key_store(),
                                pm,
                            )
                        })
                        .await
                        .transpose()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unable to encrypt payment method data")?;

                let pm_network_token_data_encrypted: Option<
                    Encryptable<Secret<serde_json::Value>>,
                > = match network_token_resp {
                    Some(token_resp) => {
                        let pm_token_details = token_resp.card.as_ref().map(|card| {
                            PaymentMethodsData::Card(CardDetailsPaymentMethod::from((
                                card.clone(),
                                None,
                            )))
                        });

                        pm_token_details
                            .async_map(|pm_card| {
                                create_encrypted_data(
                                    &key_manager_state,
                                    platform.get_processor().get_key_store(),
                                    pm_card,
                                )
                            })
                            .await
                            .transpose()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unable to encrypt payment method data")?
                    }
                    None => None,
                };

                let encrypted_payment_method_billing_address: Option<
                    Encryptable<Secret<serde_json::Value>>,
                > = payment_method_billing_address
                    .async_map(|address| {
                        create_encrypted_data(
                            &key_manager_state,
                            platform.get_processor().get_key_store(),
                            address.clone(),
                        )
                    })
                    .await
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encrypt payment method billing address")?;

                let mut payment_method_id = resp.payment_method_id.clone();
                let mut locker_id = None;
                let (external_vault_details, vault_type) = match &business_profile.external_vault_details{
                    hyperswitch_domain_models::business_profile::ExternalVaultDetails::ExternalVaultEnabled(external_vault_connector_details) => {
                        (Some(external_vault_connector_details), Some(common_enums::VaultType::External))
                    },
                    hyperswitch_domain_models::business_profile::ExternalVaultDetails::Skip => (None, Some(common_enums::VaultType::Internal)),
                };
                let external_vault_mca_id = external_vault_details
                    .map(|connector_details| connector_details.vault_connector_id.clone());

                let vault_source_details = domain::PaymentMethodVaultSourceDetails::try_from((
                    vault_type,
                    external_vault_mca_id,
                ))
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to create vault source details")?;

                match duplication_check {
                    Some(duplication_check) => match duplication_check {
                        payment_methods::transformers::DataDuplicationCheck::Duplicated => {
                            let payment_method = {
                                let existing_pm_by_pmid = db
                                    .find_payment_method(
                                        platform.get_processor().get_key_store(),
                                        &payment_method_id,
                                        platform.get_processor().get_account().storage_scheme,
                                    )
                                    .await;

                                if let Err(err) = existing_pm_by_pmid {
                                    if err.current_context().is_db_not_found() {
                                        locker_id = Some(payment_method_id.clone());
                                        let existing_pm_by_locker_id = db
                                            .find_payment_method_by_locker_id(
                                                platform.get_processor().get_key_store(),
                                                &payment_method_id,
                                                platform
                                                    .get_processor()
                                                    .get_account()
                                                    .storage_scheme,
                                            )
                                            .await;

                                        match &existing_pm_by_locker_id {
                                            Ok(pm) => {
                                                payment_method_id.clone_from(&pm.payment_method_id);
                                            }
                                            Err(_) => {
                                                payment_method_id =
                                                    generate_id(consts::ID_LENGTH, "pm")
                                            }
                                        };
                                        existing_pm_by_locker_id
                                    } else {
                                        Err(err)
                                    }
                                } else {
                                    existing_pm_by_pmid
                                }
                            };

                            resp.payment_method_id = payment_method_id;

                            match payment_method {
                                Ok(pm) => {
                                    let pm_metadata = create_payment_method_metadata(
                                        pm.metadata.as_ref(),
                                        connector_token,
                                    )?;
                                    payment_methods::cards::update_payment_method_metadata_and_last_used(
                                        platform.get_processor().get_key_store(),
                                        db,
                                        pm.clone(),
                                        pm_metadata,
                                        platform.get_processor().get_account().storage_scheme,
                                    )
                                    .await
                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Failed to add payment method in db")?;
                                }
                                Err(err) => {
                                    if err.current_context().is_db_not_found() {
                                        let pm_metadata =
                                            create_payment_method_metadata(None, connector_token)?;
                                        cards
                                            .create_payment_method(
                                                &payment_method_create_request,
                                                &customer_id,
                                                &resp.payment_method_id,
                                                locker_id,
                                                merchant_id,
                                                pm_metadata,
                                                customer_acceptance,
                                                pm_data_encrypted,
                                                None,
                                                pm_status,
                                                network_transaction_id,
                                                encrypted_payment_method_billing_address,
                                                resp.card.and_then(|card| {
                                                    card.card_network.map(|card_network| {
                                                        card_network.to_string()
                                                    })
                                                }),
                                                network_token_requestor_ref_id,
                                                network_token_locker_id,
                                                pm_network_token_data_encrypted,
                                                Some(vault_source_details),
                                            )
                                            .await
                                    } else {
                                        Err(err)
                                            .change_context(
                                                errors::ApiErrorResponse::InternalServerError,
                                            )
                                            .attach_printable("Error while finding payment method")
                                    }?;
                                }
                            };
                        }
                        payment_methods::transformers::DataDuplicationCheck::MetaDataChanged => {
                            if let Some(card) = payment_method_create_request.card.clone() {
                                let payment_method = {
                                    let existing_pm_by_pmid = db
                                        .find_payment_method(
                                            platform.get_processor().get_key_store(),
                                            &payment_method_id,
                                            platform.get_processor().get_account().storage_scheme,
                                        )
                                        .await;

                                    if let Err(err) = existing_pm_by_pmid {
                                        if err.current_context().is_db_not_found() {
                                            locker_id = Some(payment_method_id.clone());
                                            let existing_pm_by_locker_id = db
                                                .find_payment_method_by_locker_id(
                                                    platform.get_processor().get_key_store(),
                                                    &payment_method_id,
                                                    platform
                                                        .get_processor()
                                                        .get_account()
                                                        .storage_scheme,
                                                )
                                                .await;

                                            match &existing_pm_by_locker_id {
                                                Ok(pm) => {
                                                    payment_method_id
                                                        .clone_from(&pm.payment_method_id);
                                                }
                                                Err(_) => {
                                                    payment_method_id =
                                                        generate_id(consts::ID_LENGTH, "pm")
                                                }
                                            };
                                            existing_pm_by_locker_id
                                        } else {
                                            Err(err)
                                        }
                                    } else {
                                        existing_pm_by_pmid
                                    }
                                };

                                resp.payment_method_id = payment_method_id;

                                let existing_pm = match payment_method {
                                    Ok(pm) => {
                                        let mandate_details =    pm
                                        .connector_mandate_details
                                        .clone()
                                        .map(|val| {
                                            val.parse_value::<PaymentsMandateReference>(
                                                "PaymentsMandateReference",
                                            )
                                        })
                                        .transpose()
                                        .change_context(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable("Failed to deserialize to Payment Mandate Reference ")?;
                                        if let Some((mandate_details, merchant_connector_id)) =
                                            mandate_details.zip(merchant_connector_id)
                                        {
                                            let connector_mandate_details =
                                                update_connector_mandate_details_status(
                                                    merchant_connector_id,
                                                    mandate_details,
                                                    ConnectorMandateStatus::Inactive,
                                                )?;
                                            payment_methods::cards::update_payment_method_connector_mandate_details(
                                            platform.get_processor().get_key_store(),
                                            db,
                                            pm.clone(),
                                            connector_mandate_details,
                                            platform.get_processor().get_account().storage_scheme,
                                        )
                                        .await
                                        .change_context(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable("Failed to add payment method in db")?;
                                        }
                                        Ok(pm)
                                    }
                                    Err(err) => {
                                        if err.current_context().is_db_not_found() {
                                            cards
                                                .create_payment_method(
                                                    &payment_method_create_request,
                                                    &customer_id,
                                                    &resp.payment_method_id,
                                                    locker_id,
                                                    merchant_id,
                                                    resp.metadata.clone().map(|val| val.expose()),
                                                    customer_acceptance,
                                                    pm_data_encrypted,
                                                    None,
                                                    pm_status,
                                                    network_transaction_id,
                                                    encrypted_payment_method_billing_address,
                                                    resp.card.and_then(|card| {
                                                        card.card_network.map(|card_network| {
                                                            card_network.to_string()
                                                        })
                                                    }),
                                                    network_token_requestor_ref_id,
                                                    network_token_locker_id,
                                                    pm_network_token_data_encrypted,
                                                    Some(vault_source_details),
                                                )
                                                .await
                                        } else {
                                            Err(err)
                                                .change_context(
                                                    errors::ApiErrorResponse::InternalServerError,
                                                )
                                                .attach_printable(
                                                    "Error while finding payment method",
                                                )
                                        }
                                    }
                                }?;

                                cards
                                    .delete_card_from_locker(
                                        &customer_id,
                                        merchant_id,
                                        existing_pm
                                            .locker_id
                                            .as_ref()
                                            .unwrap_or(&existing_pm.payment_method_id),
                                    )
                                    .await?;

                                let add_card_resp = cards
                                    .add_card_hs(
                                        payment_method_create_request,
                                        &card,
                                        &customer_id,
                                        api::enums::LockerChoice::HyperswitchCardVault,
                                        Some(
                                            existing_pm
                                                .locker_id
                                                .as_ref()
                                                .unwrap_or(&existing_pm.payment_method_id),
                                        ),
                                    )
                                    .await;

                                if let Err(err) = add_card_resp {
                                    logger::error!(vault_err=?err);
                                    db.delete_payment_method_by_merchant_id_payment_method_id(
                                        platform.get_processor().get_key_store(),
                                        merchant_id,
                                        &resp.payment_method_id,
                                    )
                                    .await
                                    .to_not_found_response(
                                        errors::ApiErrorResponse::PaymentMethodNotFound,
                                    )?;

                                    Err(report!(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable(
                                            "Failed while updating card metadata changes",
                                        ))?
                                };

                                let existing_pm_data = cards
                                    .get_card_details_without_locker_fallback(&existing_pm)
                                    .await?;

                                // scheme should be updated in case of co-badged cards
                                let card_scheme = card
                                    .card_network
                                    .clone()
                                    .map(|card_network| card_network.to_string())
                                    .or(existing_pm_data.scheme.clone());

                                let updated_card = Some(CardDetailFromLocker {
                                    scheme: card_scheme.clone(),
                                    last4_digits: Some(card.card_number.get_last4()),
                                    issuer_country: card
                                        .card_issuing_country
                                        .or(existing_pm_data.issuer_country),
                                    card_isin: Some(card.card_number.get_card_isin()),
                                    card_number: Some(card.card_number),
                                    expiry_month: Some(card.card_exp_month),
                                    expiry_year: Some(card.card_exp_year),
                                    card_token: None,
                                    card_fingerprint: None,
                                    card_holder_name: card
                                        .card_holder_name
                                        .or(existing_pm_data.card_holder_name),
                                    nick_name: card.nick_name.or(existing_pm_data.nick_name),
                                    card_network: card
                                        .card_network
                                        .or(existing_pm_data.card_network),
                                    card_issuer: card.card_issuer.or(existing_pm_data.card_issuer),
                                    card_type: card.card_type.or(existing_pm_data.card_type),
                                    saved_to_locker: true,
                                });

                                let updated_pmd = updated_card.as_ref().map(|card| {
                                    PaymentMethodsData::Card(CardDetailsPaymentMethod::from((
                                        card.clone(),
                                        co_badged_card_data,
                                    )))
                                });
                                let pm_data_encrypted: Option<
                                    Encryptable<Secret<serde_json::Value>>,
                                > = updated_pmd
                                    .async_map(|pmd| {
                                        create_encrypted_data(
                                            &key_manager_state,
                                            platform.get_processor().get_key_store(),
                                            pmd,
                                        )
                                    })
                                    .await
                                    .transpose()
                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Unable to encrypt payment method data")?;

                                payment_methods::cards::update_payment_method_and_last_used(
                                    platform.get_processor().get_key_store(),
                                    db,
                                    existing_pm,
                                    pm_data_encrypted.map(Into::into),
                                    platform.get_processor().get_account().storage_scheme,
                                    card_scheme,
                                )
                                .await
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("Failed to add payment method in db")?;
                            }
                        }
                    },
                    None => {
                        let customer_saved_pm_option = if payment_method_type
                            .map(|payment_method_type_value| {
                                payment_method_type_value
                                    .should_check_for_customer_saved_payment_method_type()
                            })
                            .unwrap_or(false)
                        {
                            match state
                                .store
                                .find_payment_method_by_customer_id_merchant_id_list(
                                    platform.get_processor().get_key_store(),
                                    &customer_id,
                                    merchant_id,
                                    None,
                                )
                                .await
                            {
                                Ok(customer_payment_methods) => Ok(customer_payment_methods
                                    .iter()
                                    .find(|payment_method| {
                                        payment_method.get_payment_method_subtype()
                                            == payment_method_type
                                    })
                                    .cloned()),
                                Err(error) => {
                                    if error.current_context().is_db_not_found() {
                                        Ok(None)
                                    } else {
                                        Err(error)
                                            .change_context(
                                                errors::ApiErrorResponse::InternalServerError,
                                            )
                                            .attach_printable(
                                                "failed to find payment methods for a customer",
                                            )
                                    }
                                }
                            }
                        } else {
                            Ok(None)
                        }?;

                        if let Some(customer_saved_pm) = customer_saved_pm_option {
                            payment_methods::cards::update_last_used_at(
                                &customer_saved_pm,
                                state,
                                platform.get_processor().get_account().storage_scheme,
                                platform.get_processor().get_key_store(),
                            )
                            .await
                            .map_err(|e| {
                                logger::error!("Failed to update last used at: {:?}", e);
                            })
                            .ok();
                            resp.payment_method_id = customer_saved_pm.payment_method_id;
                        } else {
                            let pm_metadata =
                                create_payment_method_metadata(None, connector_token)?;

                            locker_id = resp.payment_method.and_then(|pm| {
                                if pm == PaymentMethod::Card {
                                    Some(resp.payment_method_id)
                                } else {
                                    None
                                }
                            });

                            resp.payment_method_id = generate_id(consts::ID_LENGTH, "pm");
                            cards
                                .create_payment_method(
                                    &payment_method_create_request,
                                    &customer_id,
                                    &resp.payment_method_id,
                                    locker_id,
                                    merchant_id,
                                    pm_metadata,
                                    customer_acceptance,
                                    pm_data_encrypted,
                                    None,
                                    pm_status,
                                    network_transaction_id,
                                    encrypted_payment_method_billing_address,
                                    resp.card.and_then(|card| {
                                        card.card_network
                                            .map(|card_network| card_network.to_string())
                                    }),
                                    network_token_requestor_ref_id.clone(),
                                    network_token_locker_id,
                                    pm_network_token_data_encrypted,
                                    Some(vault_source_details),
                                )
                                .await?;

                            match network_token_requestor_ref_id {
                                Some(network_token_requestor_ref_id) => {
                                    //Insert the network token reference ID along with merchant id, customer id in CallbackMapper table for its respective webooks
                                    let callback_mapper_data =
                                        CallbackMapperData::NetworkTokenWebhook {
                                            merchant_id: platform
                                                .get_processor()
                                                .get_account()
                                                .get_id()
                                                .clone(),
                                            customer_id,
                                            payment_method_id: resp.payment_method_id.clone(),
                                        };
                                    let callback_mapper = CallbackMapper::new(
                                        network_token_requestor_ref_id,
                                        common_enums::CallbackMapperIdType::NetworkTokenRequestorReferenceID,
                                        callback_mapper_data,
                                        common_utils::date_time::now(),
                                        common_utils::date_time::now(),
                                    );

                                    db.insert_call_back_mapper(callback_mapper)
                                        .await
                                        .change_context(
                                            errors::ApiErrorResponse::InternalServerError,
                                        )
                                        .attach_printable(
                                            "Failed to insert in Callback Mapper table",
                                        )?;
                                }
                                None => {
                                    logger::info!("Network token requestor reference ID is not available, skipping callback mapper insertion");
                                }
                            };
                        };
                    }
                }

                Some(resp.payment_method_id)
            } else {
                None
            };
            // check if there needs to be a config if yes then remove it to a different place
            let connector_mandate_reference_id = if connector_mandate_id.is_some() {
                if let Some(ref mut record) = original_connector_mandate_reference_id {
                    record.update(
                        connector_mandate_id,
                        None,
                        None,
                        mandate_metadata,
                        connector_mandate_request_reference_id,
                    );
                    Some(record.clone())
                } else {
                    Some(ConnectorMandateReferenceId::new(
                        connector_mandate_id,
                        None,
                        None,
                        mandate_metadata,
                        connector_mandate_request_reference_id,
                        None,
                    ))
                }
            } else {
                None
            };

            Ok(SavePaymentMethodDataResponse {
                payment_method_id: pm_id,
                payment_method_status: pm_status,
                connector_mandate_reference_id,
            })
        }
        Err(_) => Ok(SavePaymentMethodDataResponse {
            payment_method_id: None,
            payment_method_status: None,
            connector_mandate_reference_id: None,
        }),
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn save_payment_method<FData>(
    _state: &SessionState,
    _connector_name: String,
    _save_payment_method_data: SavePaymentMethodData<FData>,
    _customer_id: Option<id_type::CustomerId>,
    _merchant_context: &domain::Platform,
    _payment_method_type: Option<storage_enums::PaymentMethodType>,
    _billing_name: Option<Secret<String>>,
    _payment_method_billing_address: Option<&api::Address>,
    _business_profile: &domain::Profile,
    _connector_mandate_request_reference_id: Option<String>,
) -> RouterResult<SavePaymentMethodDataResponse>
where
    FData: mandate::MandateBehaviour + Clone,
{
    todo!()
}

#[cfg(feature = "v1")]
pub async fn pre_payment_tokenization(
    state: &SessionState,
    customer_id: id_type::CustomerId,
    card: &payment_method_data::Card,
) -> RouterResult<(Option<pm_types::TokenResponse>, Option<String>)> {
    let network_tokenization_supported_card_networks = &state
        .conf
        .network_tokenization_supported_card_networks
        .card_networks;

    if card
        .card_network
        .as_ref()
        .filter(|cn| network_tokenization_supported_card_networks.contains(cn))
        .is_some()
    {
        let optional_card_cvc = Some(card.card_cvc.clone());
        let card_detail = payment_method_data::CardDetail::from(card);
        match network_tokenization::make_card_network_tokenization_request(
            state,
            &card_detail,
            optional_card_cvc,
            &customer_id,
        )
        .await
        {
            Ok((_token_response, network_token_requestor_ref_id)) => {
                let network_tokenization_service = &state.conf.network_tokenization_service;
                match (
                    network_token_requestor_ref_id.clone(),
                    network_tokenization_service,
                ) {
                    (Some(token_ref), Some(network_tokenization_service)) => {
                        let network_token = record_operation_time(
                            async {
                                network_tokenization::get_network_token(
                                    state,
                                    customer_id,
                                    token_ref,
                                    network_tokenization_service.get_inner(),
                                )
                                .await
                            },
                            &metrics::FETCH_NETWORK_TOKEN_TIME,
                            &[],
                        )
                        .await;
                        match network_token {
                            Ok(token_response) => {
                                Ok((Some(token_response), network_token_requestor_ref_id.clone()))
                            }
                            _ => {
                                logger::error!(
                                    "Error while fetching token from tokenization service"
                                );
                                Ok((None, network_token_requestor_ref_id.clone()))
                            }
                        }
                    }
                    (Some(token_ref), _) => Ok((None, Some(token_ref))),
                    _ => Ok((None, None)),
                }
            }
            Err(err) => {
                logger::error!("Failed to tokenize card: {:?}", err);
                Ok((None, None)) //None will be returned in case of error when calling network tokenization service
            }
        }
    } else {
        Ok((None, None)) //None will be returned in case of unsupported card network.
    }
}

#[cfg(feature = "v1")]
async fn skip_saving_card_in_locker(
    platform: &domain::Platform,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    let merchant_id = platform.get_processor().get_account().get_id();
    let customer_id = payment_method_request
        .clone()
        .customer_id
        .clone()
        .get_required_value("customer_id")?;
    let payment_method_id = common_utils::generate_id(consts::ID_LENGTH, "pm");

    let last4_digits = payment_method_request
        .card
        .clone()
        .map(|c| c.card_number.get_last4());

    let card_isin = payment_method_request
        .card
        .clone()
        .map(|c| c.card_number.get_card_isin());

    match payment_method_request.card.clone() {
        Some(card) => {
            let card_detail = CardDetailFromLocker {
                scheme: None,
                issuer_country: card.card_issuing_country.clone(),
                last4_digits: last4_digits.clone(),
                card_number: None,
                expiry_month: Some(card.card_exp_month.clone()),
                expiry_year: Some(card.card_exp_year),
                card_token: None,
                card_holder_name: card.card_holder_name.clone(),
                card_fingerprint: None,
                nick_name: None,
                card_isin: card_isin.clone(),
                card_issuer: card.card_issuer.clone(),
                card_network: card.card_network.clone(),
                card_type: card.card_type.clone(),
                saved_to_locker: false,
            };
            let pm_resp = api::PaymentMethodResponse {
                merchant_id: merchant_id.to_owned(),
                customer_id: Some(customer_id),
                payment_method_id,
                payment_method: payment_method_request.payment_method,
                payment_method_type: payment_method_request.payment_method_type,
                card: Some(card_detail),
                recurring_enabled: Some(false),
                installment_payment_enabled: Some(false),
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
                metadata: None,
                created: Some(common_utils::date_time::now()),
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                last_used_at: Some(common_utils::date_time::now()),
                client_secret: None,
            };

            Ok((pm_resp, None))
        }
        None => {
            let pm_id = common_utils::generate_id(consts::ID_LENGTH, "pm");
            let payment_method_response = api::PaymentMethodResponse {
                merchant_id: merchant_id.to_owned(),
                customer_id: Some(customer_id),
                payment_method_id: pm_id,
                payment_method: payment_method_request.payment_method,
                payment_method_type: payment_method_request.payment_method_type,
                card: None,
                metadata: None,
                created: Some(common_utils::date_time::now()),
                recurring_enabled: Some(false),
                installment_payment_enabled: Some(false),
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                last_used_at: Some(common_utils::date_time::now()),
                client_secret: None,
            };
            Ok((payment_method_response, None))
        }
    }
}

#[cfg(feature = "v2")]
async fn skip_saving_card_in_locker(
    platform: &domain::Platform,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn save_in_locker_internal(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method_request: api::PaymentMethodCreate,
    card_detail: Option<api::CardDetail>,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    payment_method_request.validate()?;
    let merchant_id = platform.get_processor().get_account().get_id();
    let customer_id = payment_method_request
        .customer_id
        .clone()
        .get_required_value("customer_id")?;
    match (payment_method_request.card.clone(), card_detail) {
        (_, Some(card)) | (Some(card), _) => {
            Box::pin(PmCards { state, platform }.add_card_to_locker(
                payment_method_request,
                &card,
                &customer_id,
                None,
            ))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add Card Failed")
        }
        _ => {
            let pm_id = common_utils::generate_id(consts::ID_LENGTH, "pm");
            let payment_method_response = api::PaymentMethodResponse {
                merchant_id: merchant_id.clone(),
                customer_id: Some(customer_id),
                payment_method_id: pm_id,
                payment_method: payment_method_request.payment_method,
                payment_method_type: payment_method_request.payment_method_type,
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                card: None,
                metadata: None,
                created: Some(common_utils::date_time::now()),
                recurring_enabled: Some(false),
                installment_payment_enabled: Some(false),
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219]
                last_used_at: Some(common_utils::date_time::now()),
                client_secret: None,
            };
            Ok((payment_method_response, None))
        }
    }
}

#[cfg(feature = "v1")]
pub async fn save_in_locker_external(
    state: &SessionState,
    platform: &domain::Platform,
    payment_method_request: api::PaymentMethodCreate,
    card_detail: Option<api::CardDetail>,
    external_vault_connector_details: &ExternalVaultConnectorDetails,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    let customer_id = payment_method_request
        .customer_id
        .clone()
        .get_required_value("customer_id")?;
    // For external vault, we need to convert the card data to PaymentMethodVaultingData
    if let Some(card) = card_detail {
        let payment_method_custom_vaulting_data = get_payment_method_custom_data(
            hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(card.clone()),
            external_vault_connector_details
                .vault_token_selector
                .clone(),
        )?;

        let external_vault_mca_id = external_vault_connector_details.vault_connector_id.clone();

        let merchant_connector_account_details = state
            .store
            .find_by_merchant_connector_account_merchant_id_merchant_connector_id(
                platform.get_processor().get_account().get_id(),
                &external_vault_mca_id,
                platform.get_processor().get_key_store(),
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: external_vault_mca_id.get_string_repr().to_string(),
            })?;

        // Call vault_payment_method_external_v1
        let vault_response = Box::pin(vault_payment_method_external_v1(
            state,
            &payment_method_custom_vaulting_data,
            platform.get_processor().get_account(),
            merchant_connector_account_details,
            None,
        ))
        .await?;

        let payment_method_id = vault_response.vault_id.get_single_vault_id()?;
        let card_detail = CardDetailFromLocker::from(card);

        let pm_resp = api::PaymentMethodResponse {
            merchant_id: platform.get_processor().get_account().get_id().to_owned(),
            customer_id: Some(customer_id),
            payment_method_id,
            payment_method: payment_method_request.payment_method,
            payment_method_type: payment_method_request.payment_method_type,
            card: Some(card_detail),
            recurring_enabled: Some(false),
            installment_payment_enabled: Some(false),
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            metadata: None,
            created: Some(common_utils::date_time::now()),
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            last_used_at: Some(common_utils::date_time::now()),
            client_secret: None,
        };

        Ok((pm_resp, None))
    } else {
        //Similar implementation is done for save in locker internal
        let pm_id = common_utils::generate_id(consts::ID_LENGTH, "pm");
        let payment_method_response = api::PaymentMethodResponse {
            merchant_id: platform.get_processor().get_account().get_id().to_owned(),
            customer_id: Some(customer_id),
            payment_method_id: pm_id,
            payment_method: payment_method_request.payment_method,
            payment_method_type: payment_method_request.payment_method_type,
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            card: None,
            metadata: None,
            created: Some(common_utils::date_time::now()),
            recurring_enabled: Some(false),
            installment_payment_enabled: Some(false),
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219]
            last_used_at: Some(common_utils::date_time::now()),
            client_secret: None,
        };
        Ok((payment_method_response, None))
    }
}

#[cfg(feature = "v2")]
pub async fn save_in_locker_internal(
    _state: &SessionState,
    _platform: &domain::Platform,
    _payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    todo!()
}

#[cfg(feature = "v2")]
pub async fn save_network_token_in_locker(
    _state: &SessionState,
    _platform: &domain::Platform,
    _card_data: &domain::Card,
    _payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    Option<api_models::payment_methods::PaymentMethodResponse>,
    Option<payment_methods::transformers::DataDuplicationCheck>,
    Option<String>,
)> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn save_network_token_in_locker(
    state: &SessionState,
    platform: &domain::Platform,
    card_data: &payment_method_data::Card,
    network_token_data: Option<api::CardDetail>,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    Option<api_models::payment_methods::PaymentMethodResponse>,
    Option<payment_methods::transformers::DataDuplicationCheck>,
    Option<String>,
)> {
    let customer_id = payment_method_request
        .customer_id
        .clone()
        .get_required_value("customer_id")?;
    let network_tokenization_supported_card_networks = &state
        .conf
        .network_tokenization_supported_card_networks
        .card_networks;

    match network_token_data {
        Some(nt_data) => {
            let (res, dc) = Box::pin(PmCards { state, platform }.add_card_to_locker(
                payment_method_request,
                &nt_data,
                &customer_id,
                None,
            ))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add Network Token Failed")?;

            Ok((Some(res), dc, None))
        }
        None => {
            if card_data
                .card_network
                .as_ref()
                .filter(|cn| network_tokenization_supported_card_networks.contains(cn))
                .is_some()
            {
                let optional_card_cvc = Some(card_data.card_cvc.clone());
                match network_tokenization::make_card_network_tokenization_request(
                    state,
                    &domain::CardDetail::from(card_data),
                    optional_card_cvc,
                    &customer_id,
                )
                .await
                {
                    Ok((token_response, network_token_requestor_ref_id)) => {
                        // Only proceed if the tokenization was successful
                        let network_token_data = api::CardDetail {
                            card_number: token_response.token.clone(),
                            card_exp_month: token_response.token_expiry_month.clone(),
                            card_exp_year: token_response.token_expiry_year.clone(),
                            card_cvc: None,
                            card_holder_name: None,
                            nick_name: None,
                            card_issuing_country: None,
                            card_network: Some(token_response.card_brand.clone()),
                            card_issuer: None,
                            card_type: None,
                        };

                        let (res, dc) = Box::pin(PmCards { state, platform }.add_card_to_locker(
                            payment_method_request,
                            &network_token_data,
                            &customer_id,
                            None,
                        ))
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Add Network Token Failed")?;

                        Ok((Some(res), dc, network_token_requestor_ref_id))
                    }
                    Err(err) => {
                        logger::error!("Failed to tokenize card: {:?}", err);
                        Ok((None, None, None)) //None will be returned in case of error when calling network tokenization service
                    }
                }
            } else {
                Ok((None, None, None)) //None will be returned in case of unsupported card network.
            }
        }
    }
}

pub fn handle_tokenization_response<F, Req>(
    resp: &mut types::RouterData<F, Req, types::PaymentsResponseData>,
) {
    let response = resp.response.clone();
    if let Err(err) = response {
        if let Some(secret_metadata) = &err.connector_metadata {
            let metadata = secret_metadata.clone().expose();
            if let Some(token) = metadata
                .get("payment_method_token")
                .and_then(|t| t.as_str())
            {
                resp.response = Ok(types::PaymentsResponseData::TokenizationResponse {
                    token: token.to_string(),
                });
            }
        }
    }
}

pub fn create_payment_method_metadata(
    metadata: Option<&pii::SecretSerdeValue>,
    connector_token: Option<(String, String)>,
) -> RouterResult<Option<serde_json::Value>> {
    let mut meta = match metadata {
        None => serde_json::Map::new(),
        Some(meta) => {
            let metadata = meta.clone().expose();
            let existing_metadata: serde_json::Map<String, serde_json::Value> = metadata
                .parse_value("Map<String, Value>")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to parse the metadata")?;
            existing_metadata
        }
    };
    Ok(connector_token.and_then(|connector_and_token| {
        meta.insert(
            connector_and_token.0,
            serde_json::Value::String(connector_and_token.1),
        )
    }))
}

pub async fn add_payment_method_token<F: Clone, T: types::Tokenizable + Clone>(
    state: &SessionState,
    connector: &api::ConnectorData,
    tokenization_action: &payments::TokenizationAction,
    router_data: &mut types::RouterData<F, T, types::PaymentsResponseData>,
    pm_token_request_data: types::PaymentMethodTokenizationData,
    should_continue_payment: bool,
) -> RouterResult<types::PaymentMethodTokenResult> {
    if should_continue_payment {
        match tokenization_action {
            payments::TokenizationAction::TokenizeInConnector => {
                let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
                    api::PaymentMethodToken,
                    types::PaymentMethodTokenizationData,
                    types::PaymentsResponseData,
                > = connector.connector.get_connector_integration();

                let pm_token_response_data: Result<
                    types::PaymentsResponseData,
                    types::ErrorResponse,
                > = Err(types::ErrorResponse::default());

                let pm_token_router_data =
                    helpers::router_data_type_conversion::<_, api::PaymentMethodToken, _, _, _, _>(
                        router_data.clone(),
                        pm_token_request_data,
                        pm_token_response_data,
                    );

                router_data
                    .request
                    .set_session_token(pm_token_router_data.session_token.clone());

                let mut resp = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    &pm_token_router_data,
                    payments::CallConnectorAction::Trigger,
                    None,
                    None,
                )
                .await
                .to_payment_failed_response()?;

                // checks for metadata in the ErrorResponse, if present bypasses it and constructs an Ok response
                handle_tokenization_response(&mut resp);

                metrics::CONNECTOR_PAYMENT_METHOD_TOKENIZATION.add(
                    1,
                    router_env::metric_attributes!(
                        ("connector", connector.connector_name.to_string()),
                        ("payment_method", router_data.payment_method.to_string()),
                    ),
                );

                let payment_token_resp = resp.response.map(|res| {
                    if let types::PaymentsResponseData::TokenizationResponse { token } = res {
                        Some(token)
                    } else {
                        None
                    }
                });

                Ok(types::PaymentMethodTokenResult {
                    payment_method_token_result: payment_token_resp,
                    is_payment_method_tokenization_performed: true,
                    connector_response: resp.connector_response.clone(),
                })
            }
            _ => Ok(types::PaymentMethodTokenResult {
                payment_method_token_result: Ok(None),
                is_payment_method_tokenization_performed: false,
                connector_response: None,
            }),
        }
    } else {
        logger::debug!("Skipping connector tokenization based on should_continue_payment flag");
        Ok(types::PaymentMethodTokenResult {
            payment_method_token_result: Ok(None),
            is_payment_method_tokenization_performed: false,
            connector_response: None,
        })
    }
}

pub fn update_router_data_with_payment_method_token_result<F: Clone, T>(
    payment_method_token_result: types::PaymentMethodTokenResult,
    router_data: &mut types::RouterData<F, T, types::PaymentsResponseData>,
    is_retry_payment: bool,
    should_continue_further: bool,
) -> bool {
    if payment_method_token_result.is_payment_method_tokenization_performed {
        match payment_method_token_result.payment_method_token_result {
            Ok(pm_token_result) => {
                router_data.payment_method_token = pm_token_result.map(|pm_token| {
                    hyperswitch_domain_models::router_data::PaymentMethodToken::Token(Secret::new(
                        pm_token,
                    ))
                });
                if router_data.connector_response.is_none() {
                    router_data.connector_response =
                        payment_method_token_result.connector_response.clone();
                }
                true
            }
            Err(err) => {
                if is_retry_payment {
                    router_data.response = Err(err);
                    false
                } else {
                    logger::debug!(payment_method_tokenization_error=?err);
                    true
                }
            }
        }
    } else {
        should_continue_further
    }
}

#[cfg(feature = "v1")]
pub fn add_connector_mandate_details_in_payment_method(
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    authorized_amount: Option<i64>,
    authorized_currency: Option<storage_enums::Currency>,
    merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    connector_mandate_id: Option<String>,
    mandate_metadata: Option<Secret<serde_json::Value>>,
    connector_mandate_request_reference_id: Option<String>,
) -> Option<CommonMandateReference> {
    let mut mandate_details = HashMap::new();

    if let Some((mca_id, connector_mandate_id)) =
        merchant_connector_id.clone().zip(connector_mandate_id)
    {
        mandate_details.insert(
            mca_id,
            PaymentsMandateReferenceRecord {
                connector_mandate_id,
                payment_method_type,
                original_payment_authorized_amount: authorized_amount,
                original_payment_authorized_currency: authorized_currency,
                mandate_metadata,
                connector_mandate_status: Some(ConnectorMandateStatus::Active),
                connector_mandate_request_reference_id,
                connector_customer_id: None,
            },
        );
        Some(CommonMandateReference {
            payments: Some(PaymentsMandateReference(mandate_details)),
            payouts: None,
        })
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
#[cfg(feature = "v1")]
pub fn update_connector_mandate_details(
    mandate_details: Option<CommonMandateReference>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    authorized_amount: Option<i64>,
    authorized_currency: Option<storage_enums::Currency>,
    merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    connector_mandate_id: Option<String>,
    mandate_metadata: Option<Secret<serde_json::Value>>,
    connector_mandate_request_reference_id: Option<String>,
) -> RouterResult<Option<CommonMandateReference>> {
    let mandate_reference = match mandate_details
        .as_ref()
        .and_then(|common_mandate| common_mandate.payments.clone())
    {
        Some(mut payment_mandate_reference) => {
            if let Some((mca_id, connector_mandate_id)) =
                merchant_connector_id.clone().zip(connector_mandate_id)
            {
                let updated_record = PaymentsMandateReferenceRecord {
                    connector_mandate_id: connector_mandate_id.clone(),
                    payment_method_type,
                    original_payment_authorized_amount: authorized_amount,
                    original_payment_authorized_currency: authorized_currency,
                    mandate_metadata: mandate_metadata.clone(),
                    connector_mandate_status: Some(ConnectorMandateStatus::Active),
                    connector_mandate_request_reference_id: connector_mandate_request_reference_id
                        .clone(),
                    connector_customer_id: None,
                };

                payment_mandate_reference
                    .entry(mca_id)
                    .and_modify(|pm| *pm = updated_record)
                    .or_insert(PaymentsMandateReferenceRecord {
                        connector_mandate_id,
                        payment_method_type,
                        original_payment_authorized_amount: authorized_amount,
                        original_payment_authorized_currency: authorized_currency,
                        mandate_metadata: mandate_metadata.clone(),
                        connector_mandate_status: Some(ConnectorMandateStatus::Active),
                        connector_mandate_request_reference_id,
                        connector_customer_id: None,
                    });

                let payout_data = mandate_details.and_then(|common_mandate| common_mandate.payouts);

                Some(CommonMandateReference {
                    payments: Some(payment_mandate_reference),
                    payouts: payout_data,
                })
            } else {
                None
            }
        }
        None => add_connector_mandate_details_in_payment_method(
            payment_method_type,
            authorized_amount,
            authorized_currency,
            merchant_connector_id,
            connector_mandate_id,
            mandate_metadata,
            connector_mandate_request_reference_id,
        ),
    };
    Ok(mandate_reference)
}

#[cfg(feature = "v1")]
pub fn update_connector_mandate_details_status(
    merchant_connector_id: id_type::MerchantConnectorAccountId,
    mut payment_mandate_reference: PaymentsMandateReference,
    status: ConnectorMandateStatus,
) -> RouterResult<Option<CommonMandateReference>> {
    let mandate_reference = {
        payment_mandate_reference
            .entry(merchant_connector_id)
            .and_modify(|pm| {
                let update_rec = PaymentsMandateReferenceRecord {
                    connector_mandate_id: pm.connector_mandate_id.clone(),
                    payment_method_type: pm.payment_method_type,
                    original_payment_authorized_amount: pm.original_payment_authorized_amount,
                    original_payment_authorized_currency: pm.original_payment_authorized_currency,
                    mandate_metadata: pm.mandate_metadata.clone(),
                    connector_mandate_status: Some(status),
                    connector_mandate_request_reference_id: pm
                        .connector_mandate_request_reference_id
                        .clone(),
                    connector_customer_id: None,
                };
                *pm = update_rec
            });
        Some(payment_mandate_reference)
    };

    Ok(Some(CommonMandateReference {
        payments: mandate_reference,
        payouts: None,
    }))
}

#[cfg(feature = "v2")]
pub async fn add_token_for_payment_method(
    router_data: &mut types::RouterData<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    >,
    payment_method_data_request: types::PaymentMethodTokenizationData,
    state: SessionState,
    merchant_connector_account_details: &hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
) -> RouterResult<types::PspTokenResult> {
    let connector_id = merchant_connector_account_details.id.clone();
    let connector_data = api::ConnectorData::get_connector_by_name(
        &(state.conf.connectors),
        &merchant_connector_account_details
            .connector_name
            .to_string(),
        api::GetToken::Connector,
        Some(connector_id.clone()),
    )?;

    let connector_integration: services::BoxedPaymentConnectorIntegrationInterface<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > = connector_data.connector.get_connector_integration();

    let payment_method_token_response_data_type: Result<
        types::PaymentsResponseData,
        types::ErrorResponse,
    > = Err(types::ErrorResponse::default());

    let payment_method_token_router_data =
        helpers::router_data_type_conversion::<_, api::PaymentMethodToken, _, _, _, _>(
            router_data.clone(),
            payment_method_data_request.clone(),
            payment_method_token_response_data_type,
        );

    let connector_integration_response = services::execute_connector_processing_step(
        &state,
        connector_integration,
        &payment_method_token_router_data,
        payments::CallConnectorAction::Trigger,
        None,
        None,
    )
    .await
    .to_payment_failed_response()?;
    let payment_token_response = connector_integration_response.response.map(|res| {
        if let types::PaymentsResponseData::TokenizationResponse { token } = res {
            Ok(token)
        } else {
            Err(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get token from connector")
        }
    });

    match payment_token_response {
        Ok(token) => Ok(types::PspTokenResult { token: Ok(token?) }),
        Err(error_response) => Ok(types::PspTokenResult {
            token: Err(error_response),
        }),
    }
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
pub async fn save_card_and_network_token_in_locker(
    state: &SessionState,
    customer_id: id_type::CustomerId,
    payment_method_status: common_enums::PaymentMethodStatus,
    payment_method_data: domain::PaymentMethodData,
    vault_operation: Option<hyperswitch_domain_models::payments::VaultOperation>,
    payment_method_info: Option<domain::PaymentMethod>,
    platform: &domain::Platform,
    payment_method_create_request: api::PaymentMethodCreate,
    is_network_tokenization_enabled: bool,
    business_profile: &domain::Profile,
) -> RouterResult<(
    (
        api_models::payment_methods::PaymentMethodResponse,
        Option<payment_methods::transformers::DataDuplicationCheck>,
        Option<String>,
    ),
    Option<api_models::payment_methods::PaymentMethodResponse>,
)> {
    let network_token_requestor_reference_id = payment_method_info
        .and_then(|pm_info| pm_info.network_token_requestor_reference_id.clone());

    match vault_operation {
        Some(hyperswitch_domain_models::payments::VaultOperation::SaveCardData(card)) => {
            let card_data = api::CardDetail::from(card.card_data.clone());
            if let (Some(nt_ref_id), Some(tokenization_service)) = (
                card.network_token_req_ref_id.clone(),
                &state.conf.network_tokenization_service,
            ) {
                let _ = record_operation_time(
                    async {
                        network_tokenization::delete_network_token_from_tokenization_service(
                            state,
                            nt_ref_id.clone(),
                            &customer_id,
                            tokenization_service.get_inner(),
                        )
                        .await
                    },
                    &metrics::DELETE_NETWORK_TOKEN_TIME,
                    &[],
                )
                .await;
            }
            let (res, dc) = Box::pin(save_in_locker(
                state,
                platform,
                payment_method_create_request.to_owned(),
                Some(card_data),
                business_profile,
            ))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add Card In Locker Failed")?;

            Ok(((res, dc, None), None))
        }
        Some(hyperswitch_domain_models::payments::VaultOperation::SaveCardAndNetworkTokenData(
            save_card_and_network_token_data,
        )) => {
            let card_data =
                api::CardDetail::from(save_card_and_network_token_data.card_data.clone());

            let network_token_data = api::CardDetail::from(
                save_card_and_network_token_data
                    .network_token
                    .network_token_data
                    .clone(),
            );

            if payment_method_status == common_enums::PaymentMethodStatus::Active {
                let (res, dc) = Box::pin(save_in_locker_internal(
                    state,
                    platform,
                    payment_method_create_request.to_owned(),
                    Some(card_data),
                ))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Add Card In Locker Failed")?;

                let (network_token_resp, _dc, _) = Box::pin(save_network_token_in_locker(
                    state,
                    platform,
                    &save_card_and_network_token_data.card_data,
                    Some(network_token_data),
                    payment_method_create_request.clone(),
                ))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Add Network Token In Locker Failed")?;

                Ok((
                    (res, dc, network_token_requestor_reference_id),
                    network_token_resp,
                ))
            } else {
                if let (Some(nt_ref_id), Some(tokenization_service)) = (
                    network_token_requestor_reference_id.clone(),
                    &state.conf.network_tokenization_service,
                ) {
                    let _ = record_operation_time(
                        async {
                            network_tokenization::delete_network_token_from_tokenization_service(
                                state,
                                nt_ref_id.clone(),
                                &customer_id,
                                tokenization_service.get_inner(),
                            )
                            .await
                        },
                        &metrics::DELETE_NETWORK_TOKEN_TIME,
                        &[],
                    )
                    .await;
                }
                let (res, dc) = Box::pin(save_in_locker_internal(
                    state,
                    platform,
                    payment_method_create_request.to_owned(),
                    Some(card_data),
                ))
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Add Card In Locker Failed")?;

                Ok(((res, dc, None), None))
            }
        }
        _ => {
            let card_data = payment_method_create_request.card.clone();
            let (res, dc) = Box::pin(save_in_locker(
                state,
                platform,
                payment_method_create_request.to_owned(),
                card_data,
                business_profile,
            ))
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add Card In Locker Failed")?;

            if is_network_tokenization_enabled {
                match &payment_method_data {
                    domain::PaymentMethodData::Card(card) => {
                        let (
                            network_token_resp,
                            _network_token_duplication_check, //the duplication check is discarded, since each card has only one token, handling card duplication check will be suffice
                            network_token_requestor_ref_id,
                        ) = Box::pin(save_network_token_in_locker(
                            state,
                            platform,
                            card,
                            None,
                            payment_method_create_request.clone(),
                        ))
                        .await?;

                        Ok((
                            (res, dc, network_token_requestor_ref_id),
                            network_token_resp,
                        ))
                    }
                    _ => Ok(((res, dc, None), None)), //network_token_resp is None in case of other payment methods
                }
            } else {
                Ok(((res, dc, None), None))
            }
        }
    }
}
