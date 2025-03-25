use std::collections::HashMap;

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use api_models::payment_methods::PaymentMethodsData;
use api_models::{
    payment_methods::PaymentMethodDataWalletInfo, payments::ConnectorMandateReferenceId,
};
use common_enums::{ConnectorMandateStatus, PaymentMethod};
use common_utils::{
    crypto::Encryptable,
    ext_traits::{AsyncExt, Encode, ValueExt},
    id_type, pii,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "v1")]
use hyperswitch_domain_models::mandates::{
    CommonMandateReference, PaymentsMandateReference, PaymentsMandateReferenceRecord,
};
use masking::{ExposeInterface, Secret};
use router_env::{instrument, tracing};

use super::helpers;
use crate::{
    consts,
    core::{
        errors::{self, ConnectorErrorExt, RouterResult, StorageErrorExt},
        mandate,
        payment_methods::{self, cards::create_encrypted_data, network_tokenization},
        payments,
    },
    logger,
    routes::{metrics, SessionState},
    services,
    types::{
        self,
        api::{self, CardDetailFromLocker, CardDetailsPaymentMethod, PaymentMethodCreateExt},
        domain,
        storage::enums as storage_enums,
    },
    utils::{generate_id, OptionExt},
};

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
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn save_payment_method<FData>(
    state: &SessionState,
    connector_name: String,
    save_payment_method_data: SavePaymentMethodData<FData>,
    customer_id: Option<id_type::CustomerId>,
    merchant_account: &domain::MerchantAccount,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    key_store: &domain::MerchantKeyStore,
    billing_name: Option<Secret<String>>,
    payment_method_billing_address: Option<&hyperswitch_domain_models::address::Address>,
    business_profile: &domain::Profile,
    mut original_connector_mandate_reference_id: Option<ConnectorMandateReferenceId>,
    merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
) -> RouterResult<SavePaymentMethodDataResponse>
where
    FData: mandate::MandateBehaviour + Clone,
{
    let mut pm_status = None;
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
                .or(mandate_data_customer_acceptance.clone().map(From::from))
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
                let payment_method_create_request =
                    payment_methods::get_payment_method_create_request(
                        Some(&save_payment_method_data.request.get_payment_method_data()),
                        Some(save_payment_method_data.payment_method),
                        payment_method_type,
                        &customer_id.clone(),
                        billing_name,
                        payment_method_billing_address,
                    )
                    .await?;
                let customer_id = customer_id.to_owned().get_required_value("customer_id")?;
                let merchant_id = merchant_account.get_id();
                let is_network_tokenization_enabled =
                    business_profile.is_network_tokenization_enabled;
                let (
                    (mut resp, duplication_check, network_token_requestor_ref_id),
                    network_token_resp,
                ) = if !state.conf.locker.locker_enabled {
                    let (res, dc) = skip_saving_card_in_locker(
                        merchant_account,
                        payment_method_create_request.to_owned(),
                    )
                    .await?;
                    ((res, dc, None), None)
                } else {
                    pm_status = Some(common_enums::PaymentMethodStatus::from(
                        save_payment_method_data.attempt_status,
                    ));
                    let (res, dc) = Box::pin(save_in_locker(
                        state,
                        merchant_account,
                        payment_method_create_request.to_owned(),
                    ))
                    .await?;

                    if is_network_tokenization_enabled {
                        let pm_data = &save_payment_method_data.request.get_payment_method_data();
                        match pm_data {
                            domain::PaymentMethodData::Card(card) => {
                                let (
                                    network_token_resp,
                                    _network_token_duplication_check, //the duplication check is discarded, since each card has only one token, handling card duplication check will be suffice
                                    network_token_requestor_ref_id,
                                ) = Box::pin(save_network_token_in_locker(
                                    state,
                                    merchant_account,
                                    card,
                                    payment_method_create_request.clone(),
                                ))
                                .await?;

                                (
                                    (res, dc, network_token_requestor_ref_id),
                                    network_token_resp,
                                )
                            }
                            _ => ((res, dc, None), None), //network_token_resp is None in case of other payment methods
                        }
                    } else {
                        ((res, dc, None), None)
                    }
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

                let optional_pm_details = match (
                    resp.card.as_ref(),
                    save_payment_method_data.request.get_payment_method_data(),
                ) {
                    (Some(card), _) => Some(PaymentMethodsData::Card(
                        CardDetailsPaymentMethod::from(card.clone()),
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
                        .async_map(|pm| create_encrypted_data(&key_manager_state, key_store, pm))
                        .await
                        .transpose()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Unable to encrypt payment method data")?;

                let pm_network_token_data_encrypted: Option<
                    Encryptable<Secret<serde_json::Value>>,
                > = match network_token_resp {
                    Some(token_resp) => {
                        let pm_token_details = token_resp.card.as_ref().map(|card| {
                            PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone()))
                        });

                        pm_token_details
                            .async_map(|pm_card| {
                                create_encrypted_data(&key_manager_state, key_store, pm_card)
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
                        create_encrypted_data(&key_manager_state, key_store, address.clone())
                    })
                    .await
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Unable to encrypt payment method billing address")?;

                let mut payment_method_id = resp.payment_method_id.clone();
                let mut locker_id = None;

                match duplication_check {
                    Some(duplication_check) => match duplication_check {
                        payment_methods::transformers::DataDuplicationCheck::Duplicated => {
                            let payment_method = {
                                let existing_pm_by_pmid = db
                                    .find_payment_method(
                                        &(state.into()),
                                        key_store,
                                        &payment_method_id,
                                        merchant_account.storage_scheme,
                                    )
                                    .await;

                                if let Err(err) = existing_pm_by_pmid {
                                    if err.current_context().is_db_not_found() {
                                        locker_id = Some(payment_method_id.clone());
                                        let existing_pm_by_locker_id = db
                                            .find_payment_method_by_locker_id(
                                                &(state.into()),
                                                key_store,
                                                &payment_method_id,
                                                merchant_account.storage_scheme,
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
                                        state,
                                        key_store,
                                        db,
                                        pm.clone(),
                                        pm_metadata,
                                        merchant_account.storage_scheme,
                                    )
                                    .await
                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Failed to add payment method in db")?;
                                }
                                Err(err) => {
                                    if err.current_context().is_db_not_found() {
                                        let pm_metadata =
                                            create_payment_method_metadata(None, connector_token)?;
                                        payment_methods::cards::create_payment_method(
                                            state,
                                            &payment_method_create_request,
                                            &customer_id,
                                            &resp.payment_method_id,
                                            locker_id,
                                            merchant_id,
                                            pm_metadata,
                                            customer_acceptance,
                                            pm_data_encrypted,
                                            key_store,
                                            None,
                                            pm_status,
                                            network_transaction_id,
                                            merchant_account.storage_scheme,
                                            encrypted_payment_method_billing_address,
                                            resp.card.and_then(|card| {
                                                card.card_network
                                                    .map(|card_network| card_network.to_string())
                                            }),
                                            network_token_requestor_ref_id,
                                            network_token_locker_id,
                                            pm_network_token_data_encrypted,
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
                                            &(state.into()),
                                            key_store,
                                            &payment_method_id,
                                            merchant_account.storage_scheme,
                                        )
                                        .await;

                                    if let Err(err) = existing_pm_by_pmid {
                                        if err.current_context().is_db_not_found() {
                                            locker_id = Some(payment_method_id.clone());
                                            let existing_pm_by_locker_id = db
                                                .find_payment_method_by_locker_id(
                                                    &(state.into()),
                                                    key_store,
                                                    &payment_method_id,
                                                    merchant_account.storage_scheme,
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
                                            state,
                                            key_store,
                                            db,
                                            pm.clone(),
                                            connector_mandate_details,
                                            merchant_account.storage_scheme,
                                        )
                                        .await
                                        .change_context(errors::ApiErrorResponse::InternalServerError)
                                        .attach_printable("Failed to add payment method in db")?;
                                        }
                                        Ok(pm)
                                    }
                                    Err(err) => {
                                        if err.current_context().is_db_not_found() {
                                            payment_methods::cards::create_payment_method(
                                                state,
                                                &payment_method_create_request,
                                                &customer_id,
                                                &resp.payment_method_id,
                                                locker_id,
                                                merchant_id,
                                                resp.metadata.clone().map(|val| val.expose()),
                                                customer_acceptance,
                                                pm_data_encrypted,
                                                key_store,
                                                None,
                                                pm_status,
                                                network_transaction_id,
                                                merchant_account.storage_scheme,
                                                encrypted_payment_method_billing_address,
                                                resp.card.and_then(|card| {
                                                    card.card_network.map(|card_network| {
                                                        card_network.to_string()
                                                    })
                                                }),
                                                network_token_requestor_ref_id,
                                                network_token_locker_id,
                                                pm_network_token_data_encrypted,
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

                                payment_methods::cards::delete_card_from_locker(
                                    state,
                                    &customer_id,
                                    merchant_id,
                                    existing_pm
                                        .locker_id
                                        .as_ref()
                                        .unwrap_or(&existing_pm.payment_method_id),
                                )
                                .await?;

                                let add_card_resp = payment_methods::cards::add_card_hs(
                                    state,
                                    payment_method_create_request,
                                    &card,
                                    &customer_id,
                                    merchant_account,
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
                                        &(state.into()),
                                        key_store,
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

                                let existing_pm_data = payment_methods::cards::get_card_details_without_locker_fallback(&existing_pm,state)
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
                                    PaymentMethodsData::Card(CardDetailsPaymentMethod::from(
                                        card.clone(),
                                    ))
                                });
                                let pm_data_encrypted: Option<
                                    Encryptable<Secret<serde_json::Value>>,
                                > = updated_pmd
                                    .async_map(|pmd| {
                                        create_encrypted_data(&key_manager_state, key_store, pmd)
                                    })
                                    .await
                                    .transpose()
                                    .change_context(errors::ApiErrorResponse::InternalServerError)
                                    .attach_printable("Unable to encrypt payment method data")?;

                                payment_methods::cards::update_payment_method_and_last_used(
                                    state,
                                    key_store,
                                    db,
                                    existing_pm,
                                    pm_data_encrypted.map(Into::into),
                                    merchant_account.storage_scheme,
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
                                    &(state.into()),
                                    key_store,
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
                                merchant_account.storage_scheme,
                                key_store,
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
                            payment_methods::cards::create_payment_method(
                                state,
                                &payment_method_create_request,
                                &customer_id,
                                &resp.payment_method_id,
                                locker_id,
                                merchant_id,
                                pm_metadata,
                                customer_acceptance,
                                pm_data_encrypted,
                                key_store,
                                None,
                                pm_status,
                                network_transaction_id,
                                merchant_account.storage_scheme,
                                encrypted_payment_method_billing_address,
                                resp.card.and_then(|card| {
                                    card.card_network
                                        .map(|card_network| card_network.to_string())
                                }),
                                network_token_requestor_ref_id,
                                network_token_locker_id,
                                pm_network_token_data_encrypted,
                            )
                            .await?;
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

// check in review
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn save_payment_method<FData>(
    _state: &SessionState,
    _connector_name: String,
    _save_payment_method_data: SavePaymentMethodData<FData>,
    _customer_id: Option<id_type::CustomerId>,
    _merchant_account: &domain::MerchantAccount,
    _payment_method_type: Option<storage_enums::PaymentMethodType>,
    _key_store: &domain::MerchantKeyStore,
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
async fn skip_saving_card_in_locker(
    merchant_account: &domain::MerchantAccount,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    let merchant_id = merchant_account.get_id();
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
                recurring_enabled: false,
                installment_payment_enabled: false,
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
                recurring_enabled: false,
                installment_payment_enabled: false,
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
async fn skip_saving_card_in_locker(
    merchant_account: &domain::MerchantAccount,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    todo!()
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn save_in_locker(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    payment_method_request.validate()?;
    let merchant_id = merchant_account.get_id();
    let customer_id = payment_method_request
        .customer_id
        .clone()
        .get_required_value("customer_id")?;
    match payment_method_request.card.clone() {
        Some(card) => Box::pin(payment_methods::cards::add_card_to_locker(
            state,
            payment_method_request,
            &card,
            &customer_id,
            merchant_account,
            None,
        ))
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Add Card Failed"),
        None => {
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
                recurring_enabled: false,           //[#219]
                installment_payment_enabled: false, //[#219]
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]), //[#219]
                last_used_at: Some(common_utils::date_time::now()),
                client_secret: None,
            };
            Ok((payment_method_response, None))
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn save_in_locker(
    _state: &SessionState,
    _merchant_account: &domain::MerchantAccount,
    _payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    api_models::payment_methods::PaymentMethodResponse,
    Option<payment_methods::transformers::DataDuplicationCheck>,
)> {
    todo!()
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn save_network_token_in_locker(
    _state: &SessionState,
    _merchant_account: &domain::MerchantAccount,
    _card_data: &domain::Card,
    _payment_method_request: api::PaymentMethodCreate,
) -> RouterResult<(
    Option<api_models::payment_methods::PaymentMethodResponse>,
    Option<payment_methods::transformers::DataDuplicationCheck>,
    Option<String>,
)> {
    todo!()
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn save_network_token_in_locker(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    card_data: &domain::Card,
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
                    card_holder_name: None,
                    nick_name: None,
                    card_issuing_country: None,
                    card_network: Some(token_response.card_brand.clone()),
                    card_issuer: None,
                    card_type: None,
                };

                let (res, dc) = Box::pin(payment_methods::cards::add_card_to_locker(
                    state,
                    payment_method_request,
                    &network_token_data,
                    &customer_id,
                    merchant_account,
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
            payments::TokenizationAction::TokenizeInConnector
            | payments::TokenizationAction::TokenizeInConnectorAndApplepayPreDecrypt(_) => {
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

                let resp = services::execute_connector_processing_step(
                    state,
                    connector_integration,
                    &pm_token_router_data,
                    payments::CallConnectorAction::Trigger,
                    None,
                )
                .await
                .to_payment_failed_response()?;

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
                })
            }
            _ => Ok(types::PaymentMethodTokenResult {
                payment_method_token_result: Ok(None),
                is_payment_method_tokenization_performed: false,
            }),
        }
    } else {
        logger::debug!("Skipping connector tokenization based on should_continue_payment flag");
        Ok(types::PaymentMethodTokenResult {
            payment_method_token_result: Ok(None),
            is_payment_method_tokenization_performed: false,
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
