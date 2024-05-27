use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    str::FromStr,
};

use api_models::{
    admin::PaymentMethodsEnabled,
    enums::{self as api_enums},
    payment_methods::{
        BankAccountTokenData, Card, CardDetailUpdate, CardDetailsPaymentMethod, CardNetworkTypes,
        CountryCodeWithName, CustomerDefaultPaymentMethodResponse, ListCountriesCurrenciesRequest,
        ListCountriesCurrenciesResponse, MaskedBankDetails, PaymentExperienceTypes,
        PaymentMethodsData, RequestPaymentMethodTypes, RequiredFieldInfo,
        ResponsePaymentMethodIntermediate, ResponsePaymentMethodTypes,
        ResponsePaymentMethodsEnabled,
    },
    payments::BankCodeResponse,
    pm_auth::PaymentMethodAuthConfig,
    surcharge_decision_configs as api_surcharge_decision_configs,
};
use cgraph::ConstraintGraph;
use common_enums::enums::MerchantStorageScheme;
use common_utils::{
    consts,
    ext_traits::{AsyncExt, Encode, StringExt, ValueExt},
    generate_id,
};
use diesel_models::{business_profile::BusinessProfile, encryption::Encryption, payment_method};
use domain::CustomerUpdate;
use error_stack::{report, ResultExt};
use euclid::{
    dssa::graph::{AnalysisContext, CgraphExt},
    frontend::dir,
};
use hyperswitch_constraint_graph as cgraph;
use kgraph_utils::transformers::IntoDirValue;
use masking::Secret;
use router_env::{instrument, tracing};
use strum::IntoEnumIterator;

use super::surcharge_decision_configs::{
    perform_surcharge_decision_management_for_payment_method_list,
    perform_surcharge_decision_management_for_saved_cards,
};
#[cfg(not(feature = "connector_choice_mca_id"))]
use crate::core::utils::get_connector_label;
use crate::{
    configs::settings,
    core::{
        errors::{self, StorageErrorExt},
        payment_methods::{
            transformers as payment_methods,
            utils::{get_merchant_pm_filter_graph, make_pm_graph, refresh_pm_filters_cache},
            vault,
        },
        payments::{
            helpers,
            routing::{self, SessionFlowRoutingInput},
        },
        utils as core_utils,
    },
    db, logger,
    pii::prelude::*,
    routes::{
        self,
        metrics::{self, request},
        payment_methods::ParentPaymentMethodToken,
    },
    services,
    types::{
        api::{self, routing as routing_types, PaymentMethodCreateExt},
        domain::{
            self,
            types::{decrypt, encrypt_optional, AsyncLift},
        },
        storage::{self, enums, PaymentMethodListContext, PaymentTokenData},
        transformers::ForeignFrom,
    },
    utils::{self, ConnectorResponseExt, OptionExt},
};

#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method(
    db: &dyn db::StorageInterface,
    req: &api::PaymentMethodCreate,
    customer_id: &str,
    payment_method_id: &str,
    locker_id: Option<String>,
    merchant_id: &str,
    pm_metadata: Option<serde_json::Value>,
    customer_acceptance: Option<serde_json::Value>,
    payment_method_data: Option<Encryption>,
    key_store: &domain::MerchantKeyStore,
    connector_mandate_details: Option<serde_json::Value>,
    status: Option<enums::PaymentMethodStatus>,
    network_transaction_id: Option<String>,
    storage_scheme: MerchantStorageScheme,
    payment_method_billing_address: Option<Encryption>,
) -> errors::CustomResult<storage::PaymentMethod, errors::ApiErrorResponse> {
    let customer = db
        .find_customer_by_customer_id_merchant_id(
            customer_id,
            merchant_id,
            key_store,
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let client_secret = generate_id(
        consts::ID_LENGTH,
        format!("{payment_method_id}_secret").as_str(),
    );

    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            storage::PaymentMethodNew {
                customer_id: customer_id.to_string(),
                merchant_id: merchant_id.to_string(),
                payment_method_id: payment_method_id.to_string(),
                locker_id,
                payment_method: req.payment_method,
                payment_method_type: req.payment_method_type,
                payment_method_issuer: req.payment_method_issuer.clone(),
                scheme: req.card_network.clone(),
                metadata: pm_metadata.map(Secret::new),
                payment_method_data,
                connector_mandate_details,
                customer_acceptance: customer_acceptance.map(Secret::new),
                client_secret: Some(client_secret),
                status: status.unwrap_or(enums::PaymentMethodStatus::Active),
                network_transaction_id: network_transaction_id.to_owned(),
                payment_method_issuer_code: None,
                accepted_currency: None,
                token: None,
                cardholder_name: None,
                issuer_name: None,
                issuer_country: None,
                payer_country: None,
                is_stored: None,
                swift_code: None,
                direct_debit_token: None,
                created_at: current_time,
                last_modified: current_time,
                last_used_at: current_time,
                payment_method_billing_address,
                updated_by: None,
            },
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    if customer.default_payment_method_id.is_none() && req.payment_method.is_some() {
        let _ = set_default_payment_method(
            db,
            merchant_id.to_string(),
            key_store.clone(),
            customer_id,
            payment_method_id.to_owned(),
            storage_scheme,
        )
        .await
        .map_err(|err| logger::error!(error=?err,"Failed to set the payment method as default"));
    }
    Ok(response)
}

pub fn store_default_payment_method(
    req: &api::PaymentMethodCreate,
    customer_id: &str,
    merchant_id: &String,
) -> (
    api::PaymentMethodResponse,
    Option<payment_methods::DataDuplicationCheck>,
) {
    let pm_id = generate_id(consts::ID_LENGTH, "pm");
    let payment_method_response = api::PaymentMethodResponse {
        merchant_id: merchant_id.to_string(),
        customer_id: Some(customer_id.to_owned()),
        payment_method_id: pm_id,
        payment_method: req.payment_method,
        payment_method_type: req.payment_method_type,
        #[cfg(feature = "payouts")]
        bank_transfer: None,
        card: None,
        metadata: req.metadata.clone(),
        created: Some(common_utils::date_time::now()),
        recurring_enabled: false,           //[#219]
        installment_payment_enabled: false, //[#219]
        payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
        last_used_at: Some(common_utils::date_time::now()),
        client_secret: None,
    };

    (payment_method_response, None)
}
#[instrument(skip_all)]
pub async fn get_or_insert_payment_method(
    db: &dyn db::StorageInterface,
    req: api::PaymentMethodCreate,
    resp: &mut api::PaymentMethodResponse,
    merchant_account: &domain::MerchantAccount,
    customer_id: &str,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResult<diesel_models::PaymentMethod> {
    let mut payment_method_id = resp.payment_method_id.clone();
    let mut locker_id = None;
    let payment_method = {
        let existing_pm_by_pmid = db
            .find_payment_method(&payment_method_id, merchant_account.storage_scheme)
            .await;

        if let Err(err) = existing_pm_by_pmid {
            if err.current_context().is_db_not_found() {
                locker_id = Some(payment_method_id.clone());
                let existing_pm_by_locker_id = db
                    .find_payment_method_by_locker_id(
                        &payment_method_id,
                        merchant_account.storage_scheme,
                    )
                    .await;

                match &existing_pm_by_locker_id {
                    Ok(pm) => payment_method_id.clone_from(&pm.payment_method_id),
                    Err(_) => payment_method_id = generate_id(consts::ID_LENGTH, "pm"),
                };
                existing_pm_by_locker_id
            } else {
                Err(err)
            }
        } else {
            existing_pm_by_pmid
        }
    };
    payment_method_id.clone_into(&mut resp.payment_method_id);

    match payment_method {
        Ok(pm) => Ok(pm),
        Err(err) => {
            if err.current_context().is_db_not_found() {
                insert_payment_method(
                    db,
                    resp,
                    req,
                    key_store,
                    &merchant_account.merchant_id,
                    customer_id,
                    resp.metadata.clone().map(|val| val.expose()),
                    None,
                    locker_id,
                    None,
                    None,
                    merchant_account.storage_scheme,
                    None,
                )
                .await
            } else {
                Err(err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Error while finding payment method")
            }
        }
    }
}

#[instrument(skip_all)]
pub async fn get_client_secret_or_add_payment_method(
    state: routes::AppState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;

    #[cfg(not(feature = "payouts"))]
    let condition = req.card.is_some();
    #[cfg(feature = "payouts")]
    let condition = req.card.is_some() || req.bank_transfer.is_some() || req.wallet.is_some();

    if condition {
        add_payment_method(state, req, merchant_account, key_store).await
    } else {
        let payment_method_id = generate_id(consts::ID_LENGTH, "pm");

        let res = create_payment_method(
            db,
            &req,
            customer_id.as_str(),
            payment_method_id.as_str(),
            None,
            merchant_id.as_str(),
            None,
            None,
            None,
            key_store,
            None,
            Some(enums::PaymentMethodStatus::AwaitingData),
            None,
            merchant_account.storage_scheme,
            None,
        )
        .await?;

        Ok(services::api::ApplicationResponse::Json(
            api::PaymentMethodResponse::foreign_from(res),
        ))
    }
}

#[instrument(skip_all)]
pub fn authenticate_pm_client_secret_and_check_expiry(
    req_client_secret: &String,
    payment_method: &diesel_models::PaymentMethod,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    let stored_client_secret = payment_method
        .client_secret
        .clone()
        .get_required_value("client_secret")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })
        .attach_printable("client secret not found in db")?;

    if req_client_secret != &stored_client_secret {
        Err((errors::ApiErrorResponse::ClientSecretInvalid).into())
    } else {
        let current_timestamp = common_utils::date_time::now();
        let session_expiry = payment_method
            .created_at
            .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

        let expired = current_timestamp > session_expiry;

        Ok(expired)
    }
}

#[instrument(skip_all)]
pub async fn add_payment_method_data(
    state: routes::AppState,
    req: api::PaymentMethodCreate,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    pm_id: String,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;

    let pmd = req
        .payment_method_data
        .clone()
        .get_required_value("payment_method_data")?;
    req.payment_method.get_required_value("payment_method")?;
    let client_secret = req
        .client_secret
        .clone()
        .get_required_value("client_secret")?;
    let payment_method = db
        .find_payment_method(pm_id.as_str(), merchant_account.storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    if payment_method.status != enums::PaymentMethodStatus::AwaitingData {
        return Err((errors::ApiErrorResponse::DuplicatePaymentMethod).into());
    }

    let customer_id = payment_method.customer_id.clone();
    let customer = db
        .find_customer_by_customer_id_merchant_id(
            customer_id.as_str(),
            &merchant_account.merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let client_secret_expired =
        authenticate_pm_client_secret_and_check_expiry(&client_secret, &payment_method)?;

    if client_secret_expired {
        return Err((errors::ApiErrorResponse::ClientSecretExpired).into());
    };

    match pmd {
        api_models::payment_methods::PaymentMethodCreateData::Card(card) => {
            helpers::validate_card_expiry(&card.card_exp_month, &card.card_exp_year)?;
            let resp = add_card_to_locker(
                &state,
                req.clone(),
                &card,
                &customer_id,
                &merchant_account,
                None,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError);

            match resp {
                Ok((mut pm_resp, duplication_check)) => {
                    if duplication_check.is_some() {
                        let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                            status: Some(enums::PaymentMethodStatus::Inactive),
                        };

                        db.update_payment_method(
                            payment_method,
                            pm_update,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to add payment method in db")?;

                        get_or_insert_payment_method(
                            db,
                            req.clone(),
                            &mut pm_resp,
                            &merchant_account,
                            &customer_id,
                            &key_store,
                        )
                        .await?;

                        return Ok(services::ApplicationResponse::Json(pm_resp));
                    } else {
                        let locker_id = pm_resp.payment_method_id.clone();
                        pm_resp.payment_method_id.clone_from(&pm_id);
                        pm_resp.client_secret = Some(client_secret.clone());

                        let card_isin = card.card_number.get_card_isin();

                        let card_info = db
                            .get_card_info(card_isin.as_str())
                            .await
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to get card info")?;

                        let updated_card = CardDetailsPaymentMethod {
                            issuer_country: card_info
                                .as_ref()
                                .and_then(|ci| ci.card_issuing_country.clone()),
                            last4_digits: Some(card.card_number.get_last4()),
                            expiry_month: Some(card.card_exp_month),
                            expiry_year: Some(card.card_exp_year),
                            nick_name: card.nick_name,
                            card_holder_name: card.card_holder_name,
                            card_network: card_info.as_ref().and_then(|ci| ci.card_network.clone()),
                            card_isin: Some(card_isin),
                            card_issuer: card_info.as_ref().and_then(|ci| ci.card_issuer.clone()),
                            card_type: card_info.as_ref().and_then(|ci| ci.card_type.clone()),
                            saved_to_locker: true,
                        };

                        let updated_pmd = Some(PaymentMethodsData::Card(updated_card));
                        let pm_data_encrypted =
                            create_encrypted_data(&key_store, updated_pmd).await;

                        let pm_update = storage::PaymentMethodUpdate::AdditionalDataUpdate {
                            payment_method_data: pm_data_encrypted,
                            status: Some(enums::PaymentMethodStatus::Active),
                            locker_id: Some(locker_id),
                            payment_method: req.payment_method,
                            payment_method_issuer: req.payment_method_issuer,
                            payment_method_type: req.payment_method_type,
                        };

                        db.update_payment_method(
                            payment_method,
                            pm_update,
                            merchant_account.storage_scheme,
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to add payment method in db")?;

                        if customer.default_payment_method_id.is_none() {
                            let _ = set_default_payment_method(
                                db,
                                merchant_account.merchant_id.clone(),
                                key_store.clone(),
                                customer_id.as_str(),
                               pm_id,
                               merchant_account.storage_scheme,
                            )
                            .await
                            .map_err(|err| logger::error!(error=?err,"Failed to set the payment method as default"));
                        }

                        return Ok(services::ApplicationResponse::Json(pm_resp));
                    }
                }
                Err(e) => {
                    let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                        status: Some(enums::PaymentMethodStatus::Inactive),
                    };

                    db.update_payment_method(
                        payment_method,
                        pm_update,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to update payment method in db")?;

                    return Err(e.attach_printable("Failed to add card to locker"));
                }
            }
        }
    }
}

#[instrument(skip_all)]
pub async fn add_payment_method(
    state: routes::AppState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let customer_id = req.customer_id.clone().get_required_value("customer_id")?;
    let payment_method = req.payment_method.get_required_value("payment_method")?;

    let response = match payment_method {
        #[cfg(feature = "payouts")]
        api_enums::PaymentMethod::BankTransfer => match req.bank_transfer.clone() {
            Some(bank) => add_bank_to_locker(
                &state,
                req.clone(),
                merchant_account,
                key_store,
                &bank,
                &customer_id,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Add PaymentMethod Failed"),
            _ => Ok(store_default_payment_method(
                &req,
                &customer_id,
                merchant_id,
            )),
        },
        api_enums::PaymentMethod::Card => match req.card.clone() {
            Some(card) => {
                helpers::validate_card_expiry(&card.card_exp_month, &card.card_exp_year)?;
                add_card_to_locker(
                    &state,
                    req.clone(),
                    &card,
                    &customer_id,
                    merchant_account,
                    None,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Add Card Failed")
            }
            _ => Ok(store_default_payment_method(
                &req,
                &customer_id,
                merchant_id,
            )),
        },
        _ => Ok(store_default_payment_method(
            &req,
            &customer_id,
            merchant_id,
        )),
    };

    let (mut resp, duplication_check) = response?;

    match duplication_check {
        Some(duplication_check) => match duplication_check {
            payment_methods::DataDuplicationCheck::Duplicated => {
                let existing_pm = get_or_insert_payment_method(
                    db,
                    req.clone(),
                    &mut resp,
                    merchant_account,
                    &customer_id,
                    key_store,
                )
                .await?;

                resp.client_secret = existing_pm.client_secret;
            }
            payment_methods::DataDuplicationCheck::MetaDataChanged => {
                if let Some(card) = req.card.clone() {
                    let existing_pm = get_or_insert_payment_method(
                        db,
                        req.clone(),
                        &mut resp,
                        merchant_account,
                        &customer_id,
                        key_store,
                    )
                    .await?;

                    let client_secret = existing_pm.client_secret.clone();

                    delete_card_from_locker(
                        &state,
                        &customer_id,
                        merchant_id,
                        existing_pm
                            .locker_id
                            .as_ref()
                            .unwrap_or(&existing_pm.payment_method_id),
                    )
                    .await?;

                    let add_card_resp = add_card_hs(
                        &state,
                        req.clone(),
                        &card,
                        customer_id.clone(),
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
                            merchant_id,
                            &resp.payment_method_id,
                        )
                        .await
                        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

                        Err(report!(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed while updating card metadata changes"))?
                    };

                    let updated_card = Some(api::CardDetailFromLocker {
                        scheme: None,
                        last4_digits: Some(card.card_number.get_last4()),
                        issuer_country: None,
                        card_number: Some(card.card_number),
                        expiry_month: Some(card.card_exp_month),
                        expiry_year: Some(card.card_exp_year),
                        card_token: None,
                        card_fingerprint: None,
                        card_holder_name: card.card_holder_name,
                        nick_name: card.nick_name,
                        card_network: None,
                        card_isin: None,
                        card_issuer: None,
                        card_type: None,
                        saved_to_locker: true,
                    });

                    let updated_pmd = updated_card.as_ref().map(|card| {
                        PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone()))
                    });
                    let pm_data_encrypted = create_encrypted_data(key_store, updated_pmd).await;

                    let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
                        payment_method_data: pm_data_encrypted,
                    };

                    db.update_payment_method(
                        existing_pm,
                        pm_update,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add payment method in db")?;

                    resp.client_secret = client_secret;
                }
            }
        },
        None => {
            let pm_metadata = resp.metadata.as_ref().map(|data| data.peek());

            let locker_id = if resp.payment_method == Some(api_enums::PaymentMethod::Card)
                || resp.payment_method == Some(api_enums::PaymentMethod::BankTransfer)
            {
                Some(resp.payment_method_id)
            } else {
                None
            };
            resp.payment_method_id = generate_id(consts::ID_LENGTH, "pm");
            let pm = insert_payment_method(
                db,
                &resp,
                req,
                key_store,
                merchant_id,
                &customer_id,
                pm_metadata.cloned(),
                None,
                locker_id,
                None,
                None,
                merchant_account.storage_scheme,
                None,
            )
            .await?;

            resp.client_secret = pm.client_secret;
        }
    }

    Ok(services::ApplicationResponse::Json(resp))
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_payment_method(
    db: &dyn db::StorageInterface,
    resp: &api::PaymentMethodResponse,
    req: api::PaymentMethodCreate,
    key_store: &domain::MerchantKeyStore,
    merchant_id: &str,
    customer_id: &str,
    pm_metadata: Option<serde_json::Value>,
    customer_acceptance: Option<serde_json::Value>,
    locker_id: Option<String>,
    connector_mandate_details: Option<serde_json::Value>,
    network_transaction_id: Option<String>,
    storage_scheme: MerchantStorageScheme,
    payment_method_billing_address: Option<Encryption>,
) -> errors::RouterResult<diesel_models::PaymentMethod> {
    let pm_card_details = resp
        .card
        .as_ref()
        .map(|card| PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone())));
    let pm_data_encrypted = create_encrypted_data(key_store, pm_card_details).await;
    create_payment_method(
        db,
        &req,
        customer_id,
        &resp.payment_method_id,
        locker_id,
        merchant_id,
        pm_metadata,
        customer_acceptance,
        pm_data_encrypted,
        key_store,
        connector_mandate_details,
        None,
        network_transaction_id,
        storage_scheme,
        payment_method_billing_address,
    )
    .await
}

#[instrument(skip_all)]
pub async fn update_customer_payment_method(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    req: api::PaymentMethodUpdate,
    payment_method_id: &str,
    key_store: domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    // Currently update is supported only for cards
    if let Some(card_update) = req.card.clone() {
        let db = state.store.as_ref();

        let pm = db
            .find_payment_method(payment_method_id, merchant_account.storage_scheme)
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

        if let Some(cs) = &req.client_secret {
            let is_client_secret_expired = authenticate_pm_client_secret_and_check_expiry(cs, &pm)?;

            if is_client_secret_expired {
                return Err((errors::ApiErrorResponse::ClientSecretExpired).into());
            };
        };

        if pm.status == enums::PaymentMethodStatus::AwaitingData {
            return Err(report!(errors::ApiErrorResponse::NotSupported {
                message: "Payment method is awaiting data so it cannot be updated".into()
            }));
        }

        if pm.payment_method_data.is_none() {
            return Err(report!(errors::ApiErrorResponse::GenericNotFoundError {
                message: "payment_method_data not found".to_string()
            }));
        }

        // Fetch the existing payment method data from db
        let existing_card_data = decrypt::<serde_json::Value, masking::WithType>(
            pm.payment_method_data.clone(),
            key_store.key.get_inner().peek(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to decrypt card details")?
        .map(|x| x.into_inner().expose())
        .map(
            |value| -> Result<PaymentMethodsData, error_stack::Report<errors::ApiErrorResponse>> {
                value
                    .parse_value::<PaymentMethodsData>("PaymentMethodsData")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to deserialize payment methods data")
            },
        )
        .transpose()?
        .and_then(|pmd| match pmd {
            PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
            _ => None,
        })
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain decrypted card object from db")?;

        let is_card_updation_required =
            validate_payment_method_update(card_update.clone(), existing_card_data.clone());

        let response = if is_card_updation_required {
            // Fetch the existing card data from locker for getting card number
            let card_data_from_locker = get_card_from_locker(
                &state,
                &pm.customer_id,
                &pm.merchant_id,
                pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting card from locker")?;

            if card_update.card_exp_month.is_some() || card_update.card_exp_year.is_some() {
                helpers::validate_card_expiry(
                    card_update
                        .card_exp_month
                        .as_ref()
                        .unwrap_or(&card_data_from_locker.card_exp_month),
                    card_update
                        .card_exp_year
                        .as_ref()
                        .unwrap_or(&card_data_from_locker.card_exp_year),
                )?;
            }

            let updated_card_details = card_update.apply(card_data_from_locker.clone());

            // Construct new payment method object from request
            let new_pm = api::PaymentMethodCreate {
                payment_method: pm.payment_method,
                payment_method_type: pm.payment_method_type,
                payment_method_issuer: pm.payment_method_issuer.clone(),
                payment_method_issuer_code: pm.payment_method_issuer_code,
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                card: Some(updated_card_details.clone()),
                #[cfg(feature = "payouts")]
                wallet: None,
                metadata: None,
                customer_id: Some(pm.customer_id.clone()),
                client_secret: pm.client_secret.clone(),
                payment_method_data: None,
                card_network: None,
            };
            new_pm.validate()?;

            // Delete old payment method from locker
            delete_card_from_locker(
                &state,
                &pm.customer_id,
                &pm.merchant_id,
                pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
            )
            .await?;

            // Add the updated payment method data to locker
            let (mut add_card_resp, _) = add_card_to_locker(
                &state,
                new_pm.clone(),
                &updated_card_details,
                &pm.customer_id,
                &merchant_account,
                Some(pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id)),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add updated payment method to locker")?;

            // Construct new updated card object. Consider a field if passed in request or else populate it with the existing value from existing_card_data
            let updated_card = Some(api::CardDetailFromLocker {
                scheme: existing_card_data.scheme,
                last4_digits: Some(card_data_from_locker.card_number.get_last4()),
                issuer_country: existing_card_data.issuer_country,
                card_number: existing_card_data.card_number,
                expiry_month: card_update
                    .card_exp_month
                    .or(existing_card_data.expiry_month),
                expiry_year: card_update.card_exp_year.or(existing_card_data.expiry_year),
                card_token: existing_card_data.card_token,
                card_fingerprint: existing_card_data.card_fingerprint,
                card_holder_name: card_update
                    .card_holder_name
                    .or(existing_card_data.card_holder_name),
                nick_name: card_update.nick_name.or(existing_card_data.nick_name),
                card_network: existing_card_data.card_network,
                card_isin: existing_card_data.card_isin,
                card_issuer: existing_card_data.card_issuer,
                card_type: existing_card_data.card_type,
                saved_to_locker: true,
            });

            let updated_pmd = updated_card
                .as_ref()
                .map(|card| PaymentMethodsData::Card(CardDetailsPaymentMethod::from(card.clone())));
            let pm_data_encrypted = create_encrypted_data(&key_store, updated_pmd).await;

            let pm_update = storage::PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data: pm_data_encrypted,
            };

            add_card_resp
                .payment_method_id
                .clone_from(&pm.payment_method_id);

            db.update_payment_method(pm, pm_update, merchant_account.storage_scheme)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update payment method in db")?;

            add_card_resp
        } else {
            // Return existing payment method data as response without any changes
            api::PaymentMethodResponse {
                merchant_id: pm.merchant_id.to_owned(),
                customer_id: Some(pm.customer_id),
                payment_method_id: pm.payment_method_id,
                payment_method: pm.payment_method,
                payment_method_type: pm.payment_method_type,
                #[cfg(feature = "payouts")]
                bank_transfer: None,
                card: Some(existing_card_data),
                metadata: pm.metadata,
                created: Some(pm.created_at),
                recurring_enabled: false,
                installment_payment_enabled: false,
                payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
                last_used_at: Some(common_utils::date_time::now()),
                client_secret: pm.client_secret.clone(),
            }
        };

        Ok(services::ApplicationResponse::Json(response))
    } else {
        Err(report!(errors::ApiErrorResponse::NotSupported {
            message: "Payment method update for the given payment method is not supported".into()
        }))
    }
}

pub fn validate_payment_method_update(
    card_updation_obj: CardDetailUpdate,
    existing_card_data: api::CardDetailFromLocker,
) -> bool {
    // Return true If any one of the below condition returns true,
    // If a field is not passed in the update request, return false.
    // If the field is present, it depends on the existing field data:
    // - If existing field data is not present, or if it is present and doesn't match
    //   the update request data, then return true.
    // - Or else return false
    card_updation_obj
        .card_exp_month
        .map(|exp_month| exp_month.expose())
        .map_or(false, |new_exp_month| {
            existing_card_data
                .expiry_month
                .map(|exp_month| exp_month.expose())
                .map_or(true, |old_exp_month| new_exp_month != old_exp_month)
        })
        || card_updation_obj
            .card_exp_year
            .map(|exp_year| exp_year.expose())
            .map_or(false, |new_exp_year| {
                existing_card_data
                    .expiry_year
                    .map(|exp_year| exp_year.expose())
                    .map_or(true, |old_exp_year| new_exp_year != old_exp_year)
            })
        || card_updation_obj
            .card_holder_name
            .map(|name| name.expose())
            .map_or(false, |new_card_holder_name| {
                existing_card_data
                    .card_holder_name
                    .map(|name| name.expose())
                    .map_or(true, |old_card_holder_name| {
                        new_card_holder_name != old_card_holder_name
                    })
            })
        || card_updation_obj
            .nick_name
            .map(|nick_name| nick_name.expose())
            .map_or(false, |new_nick_name| {
                existing_card_data
                    .nick_name
                    .map(|nick_name| nick_name.expose())
                    .map_or(true, |old_nick_name| new_nick_name != old_nick_name)
            })
}

// Wrapper function to switch lockers

#[cfg(feature = "payouts")]
pub async fn add_bank_to_locker(
    state: &routes::AppState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    bank: &api::BankPayout,
    customer_id: &String,
) -> errors::CustomResult<
    (
        api::PaymentMethodResponse,
        Option<payment_methods::DataDuplicationCheck>,
    ),
    errors::VaultError,
> {
    let key = key_store.key.get_inner().peek();
    let payout_method_data = api::PayoutMethodData::Bank(bank.clone());
    let enc_data = async {
        serde_json::to_value(payout_method_data.to_owned())
            .map_err(|err| {
                logger::error!("Error while encoding payout method data: {}", err);
                errors::VaultError::SavePaymentMethodFailed
            })
            .change_context(errors::VaultError::SavePaymentMethodFailed)
            .attach_printable("Unable to encode payout method data")
            .ok()
            .map(|v| {
                let secret: Secret<String> = Secret::new(v.to_string());
                secret
            })
            .async_lift(|inner| encrypt_optional(inner, key))
            .await
    }
    .await
    .change_context(errors::VaultError::SavePaymentMethodFailed)
    .attach_printable("Failed to encrypt payout method data")?
    .map(Encryption::from)
    .map(|e| e.into_inner())
    .map_or(Err(errors::VaultError::SavePaymentMethodFailed), |e| {
        Ok(hex::encode(e.peek()))
    })?;

    let payload =
        payment_methods::StoreLockerReq::LockerGeneric(payment_methods::StoreGenericReq {
            merchant_id: &merchant_account.merchant_id,
            merchant_customer_id: customer_id.to_owned(),
            enc_data,
        });
    let store_resp = call_to_locker_hs(
        state,
        &payload,
        customer_id,
        api_enums::LockerChoice::HyperswitchCardVault,
    )
    .await?;
    let payment_method_resp = payment_methods::mk_add_bank_response_hs(
        bank.clone(),
        store_resp.card_reference,
        req,
        &merchant_account.merchant_id,
    );
    Ok((payment_method_resp, store_resp.duplication_check))
}

/// The response will be the tuple of PaymentMethodResponse and the duplication check of payment_method
pub async fn add_card_to_locker(
    state: &routes::AppState,
    req: api::PaymentMethodCreate,
    card: &api::CardDetail,
    customer_id: &String,
    merchant_account: &domain::MerchantAccount,
    card_reference: Option<&str>,
) -> errors::CustomResult<
    (
        api::PaymentMethodResponse,
        Option<payment_methods::DataDuplicationCheck>,
    ),
    errors::VaultError,
> {
    metrics::STORED_TO_LOCKER.add(&metrics::CONTEXT, 1, &[]);
    let add_card_to_hs_resp = request::record_operation_time(
        async {
            add_card_hs(
                state,
                req.clone(),
                card,
                customer_id.to_string(),
                merchant_account,
                api_enums::LockerChoice::HyperswitchCardVault,
                card_reference,
            )
            .await
            .map_err(|error| {
                metrics::CARD_LOCKER_FAILURES.add(
                    &metrics::CONTEXT,
                    1,
                    &[
                        router_env::opentelemetry::KeyValue::new("locker", "rust"),
                        router_env::opentelemetry::KeyValue::new("operation", "add"),
                    ],
                );
                error
            })
        },
        &metrics::CARD_ADD_TIME,
        &[router_env::opentelemetry::KeyValue::new("locker", "rust")],
    )
    .await?;

    logger::debug!("card added to hyperswitch-card-vault");
    Ok(add_card_to_hs_resp)
}

pub async fn get_card_from_locker(
    state: &routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
) -> errors::RouterResult<Card> {
    metrics::GET_FROM_LOCKER.add(&metrics::CONTEXT, 1, &[]);

    let get_card_from_rs_locker_resp = request::record_operation_time(
        async {
            get_card_from_hs_locker(
                state,
                customer_id,
                merchant_id,
                card_reference,
                api_enums::LockerChoice::HyperswitchCardVault,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while getting card from hyperswitch card vault")
            .map_err(|error| {
                metrics::CARD_LOCKER_FAILURES.add(
                    &metrics::CONTEXT,
                    1,
                    &[
                        router_env::opentelemetry::KeyValue::new("locker", "rust"),
                        router_env::opentelemetry::KeyValue::new("operation", "get"),
                    ],
                );
                error
            })
        },
        &metrics::CARD_GET_TIME,
        &[router_env::opentelemetry::KeyValue::new("locker", "rust")],
    )
    .await?;

    logger::debug!("card retrieved from rust locker");
    Ok(get_card_from_rs_locker_resp)
}

pub async fn delete_card_from_locker(
    state: &routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &str,
) -> errors::RouterResult<payment_methods::DeleteCardResp> {
    metrics::DELETE_FROM_LOCKER.add(&metrics::CONTEXT, 1, &[]);

    request::record_operation_time(
        async move {
            delete_card_from_hs_locker(state, customer_id, merchant_id, card_reference)
                .await
                .map_err(|error| {
                    metrics::CARD_LOCKER_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                    error
                })
        },
        &metrics::CARD_DELETE_TIME,
        &[],
    )
    .await
}

#[instrument(skip_all)]
pub async fn add_card_hs(
    state: &routes::AppState,
    req: api::PaymentMethodCreate,
    card: &api::CardDetail,
    customer_id: String,
    merchant_account: &domain::MerchantAccount,
    locker_choice: api_enums::LockerChoice,
    card_reference: Option<&str>,
) -> errors::CustomResult<
    (
        api::PaymentMethodResponse,
        Option<payment_methods::DataDuplicationCheck>,
    ),
    errors::VaultError,
> {
    let payload = payment_methods::StoreLockerReq::LockerCard(payment_methods::StoreCardReq {
        merchant_id: &merchant_account.merchant_id,
        merchant_customer_id: customer_id.to_owned(),
        requestor_card_reference: card_reference.map(str::to_string),
        card: Card {
            card_number: card.card_number.to_owned(),
            name_on_card: card.card_holder_name.to_owned(),
            card_exp_month: card.card_exp_month.to_owned(),
            card_exp_year: card.card_exp_year.to_owned(),
            card_brand: card.card_network.as_ref().map(ToString::to_string),
            card_isin: None,
            nick_name: card.nick_name.as_ref().map(Secret::peek).cloned(),
        },
    });

    let store_card_payload =
        call_to_locker_hs(state, &payload, &customer_id, locker_choice).await?;

    let payment_method_resp = payment_methods::mk_add_card_response_hs(
        card.clone(),
        store_card_payload.card_reference,
        req,
        &merchant_account.merchant_id,
    );
    Ok((payment_method_resp, store_card_payload.duplication_check))
}

#[instrument(skip_all)]
pub async fn decode_and_decrypt_locker_data(
    key_store: &domain::MerchantKeyStore,
    enc_card_data: String,
) -> errors::CustomResult<Secret<String>, errors::VaultError> {
    // Fetch key
    let key = key_store.key.get_inner().peek();
    // Decode
    let decoded_bytes = hex::decode(&enc_card_data)
        .change_context(errors::VaultError::ResponseDeserializationFailed)
        .attach_printable("Failed to decode hex string into bytes")?;
    // Decrypt
    decrypt(Some(Encryption::new(decoded_bytes.into())), key)
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)?
        .map_or(
            Err(report!(errors::VaultError::FetchPaymentMethodFailed)),
            |d| Ok(d.into_inner()),
        )
}

#[instrument(skip_all)]
pub async fn get_payment_method_from_hs_locker<'a>(
    state: &'a routes::AppState,
    key_store: &domain::MerchantKeyStore,
    customer_id: &str,
    merchant_id: &str,
    payment_method_reference: &'a str,
    locker_choice: Option<api_enums::LockerChoice>,
) -> errors::CustomResult<Secret<String>, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();

    let payment_method_data = if !locker.mock_locker {
        let request = payment_methods::mk_get_card_request_hs(
            jwekey,
            locker,
            customer_id,
            merchant_id,
            payment_method_reference,
            locker_choice,
        )
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)
        .attach_printable("Making get payment method request failed")?;
        let response = services::call_connector_api(state, request, "add_card_to_locker")
            .await
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable("Failed while executing call_connector_api for get_card");
        let jwe_body: services::JweBody = response
            .get_response_inner("JweBody")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
        let decrypted_payload =
            payment_methods::get_decrypted_response_payload(jwekey, jwe_body, locker_choice)
                .await
                .change_context(errors::VaultError::FetchPaymentMethodFailed)
                .attach_printable("Error getting decrypted response payload for get card")?;
        let get_card_resp: payment_methods::RetrieveCardResp = decrypted_payload
            .parse_struct("RetrieveCardResp")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable("Failed to parse struct to RetrieveCardResp")?;
        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable("Failed to retrieve field - payload from RetrieveCardResp")?;
        let enc_card_data = retrieve_card_resp
            .enc_card_data
            .get_required_value("enc_card_data")
            .change_context(errors::VaultError::FetchPaymentMethodFailed)
            .attach_printable(
                "Failed to retrieve field - enc_card_data from RetrieveCardRespPayload",
            )?;
        decode_and_decrypt_locker_data(key_store, enc_card_data.peek().to_string()).await?
    } else {
        mock_get_payment_method(&*state.store, key_store, payment_method_reference)
            .await?
            .payment_method
            .payment_method_data
    };
    Ok(payment_method_data)
}

#[instrument(skip_all)]
pub async fn call_to_locker_hs<'a>(
    state: &routes::AppState,
    payload: &payment_methods::StoreLockerReq<'a>,
    customer_id: &str,
    locker_choice: api_enums::LockerChoice,
) -> errors::CustomResult<payment_methods::StoreCardRespPayload, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = state.conf.jwekey.get_inner();
    let db = &*state.store;
    let stored_card_response = if !locker.mock_locker {
        let request =
            payment_methods::mk_add_locker_request_hs(jwekey, locker, payload, locker_choice)
                .await?;
        let response = services::call_connector_api(state, request, "add_card_to_hs_locker")
            .await
            .change_context(errors::VaultError::SaveCardFailed);

        let jwe_body: services::JweBody = response
            .get_response_inner("JweBody")
            .change_context(errors::VaultError::FetchCardFailed)?;

        let decrypted_payload =
            payment_methods::get_decrypted_response_payload(jwekey, jwe_body, Some(locker_choice))
                .await
                .change_context(errors::VaultError::SaveCardFailed)
                .attach_printable("Error getting decrypted response payload")?;
        let stored_card_resp: payment_methods::StoreCardResp = decrypted_payload
            .parse_struct("StoreCardResp")
            .change_context(errors::VaultError::ResponseDeserializationFailed)?;
        stored_card_resp
    } else {
        let card_id = generate_id(consts::ID_LENGTH, "card");
        mock_call_to_locker_hs(db, &card_id, payload, None, None, Some(customer_id)).await?
    };

    let stored_card = stored_card_response
        .payload
        .get_required_value("StoreCardRespPayload")
        .change_context(errors::VaultError::SaveCardFailed)?;
    Ok(stored_card)
}

pub async fn update_payment_method(
    db: &dyn db::StorageInterface,
    pm: payment_method::PaymentMethod,
    pm_metadata: serde_json::Value,
    storage_scheme: MerchantStorageScheme,
) -> errors::CustomResult<(), errors::VaultError> {
    let pm_update = payment_method::PaymentMethodUpdate::MetadataUpdate {
        metadata: Some(pm_metadata),
    };
    db.update_payment_method(pm, pm_update, storage_scheme)
        .await
        .change_context(errors::VaultError::UpdateInPaymentMethodDataTableFailed)?;
    Ok(())
}

pub async fn update_payment_method_connector_mandate_details(
    db: &dyn db::StorageInterface,
    pm: payment_method::PaymentMethod,
    connector_mandate_details: Option<serde_json::Value>,
    storage_scheme: MerchantStorageScheme,
) -> errors::CustomResult<(), errors::VaultError> {
    let pm_update = payment_method::PaymentMethodUpdate::ConnectorMandateDetailsUpdate {
        connector_mandate_details,
    };

    db.update_payment_method(pm, pm_update, storage_scheme)
        .await
        .change_context(errors::VaultError::UpdateInPaymentMethodDataTableFailed)?;
    Ok(())
}
#[instrument(skip_all)]
pub async fn get_card_from_hs_locker<'a>(
    state: &'a routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &'a str,
    locker_choice: api_enums::LockerChoice,
) -> errors::CustomResult<Card, errors::VaultError> {
    let locker = &state.conf.locker;
    let jwekey = &state.conf.jwekey.get_inner();

    if !locker.mock_locker {
        let request = payment_methods::mk_get_card_request_hs(
            jwekey,
            locker,
            customer_id,
            merchant_id,
            card_reference,
            Some(locker_choice),
        )
        .await
        .change_context(errors::VaultError::FetchCardFailed)
        .attach_printable("Making get card request failed")?;
        let response = services::call_connector_api(state, request, "get_card_from_locker")
            .await
            .change_context(errors::VaultError::FetchCardFailed)
            .attach_printable("Failed while executing call_connector_api for get_card");
        let jwe_body: services::JweBody = response
            .get_response_inner("JweBody")
            .change_context(errors::VaultError::FetchCardFailed)?;
        let decrypted_payload =
            payment_methods::get_decrypted_response_payload(jwekey, jwe_body, Some(locker_choice))
                .await
                .change_context(errors::VaultError::FetchCardFailed)
                .attach_printable("Error getting decrypted response payload for get card")?;
        let get_card_resp: payment_methods::RetrieveCardResp = decrypted_payload
            .parse_struct("RetrieveCardResp")
            .change_context(errors::VaultError::FetchCardFailed)?;
        let retrieve_card_resp = get_card_resp
            .payload
            .get_required_value("RetrieveCardRespPayload")
            .change_context(errors::VaultError::FetchCardFailed)?;
        retrieve_card_resp
            .card
            .get_required_value("Card")
            .change_context(errors::VaultError::FetchCardFailed)
    } else {
        let (get_card_resp, _) = mock_get_card(&*state.store, card_reference).await?;
        payment_methods::mk_get_card_response(get_card_resp)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
    }
}

#[instrument(skip_all)]
pub async fn delete_card_from_hs_locker<'a>(
    state: &routes::AppState,
    customer_id: &str,
    merchant_id: &str,
    card_reference: &'a str,
) -> errors::RouterResult<payment_methods::DeleteCardResp> {
    let locker = &state.conf.locker;
    let jwekey = &state.conf.jwekey.get_inner();

    let request = payment_methods::mk_delete_card_request_hs(
        jwekey,
        locker,
        customer_id,
        merchant_id,
        card_reference,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Making delete card request failed")?;

    if !locker.mock_locker {
        let response = services::call_connector_api(state, request, "delete_card_from_locker")
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed while executing call_connector_api for delete card");
        let jwe_body: services::JweBody = response.get_response_inner("JweBody")?;
        let decrypted_payload = payment_methods::get_decrypted_response_payload(
            jwekey,
            jwe_body,
            Some(api_enums::LockerChoice::HyperswitchCardVault),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting decrypted response payload for delete card")?;
        let delete_card_resp: payment_methods::DeleteCardResp = decrypted_payload
            .parse_struct("DeleteCardResp")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
        Ok(delete_card_resp)
    } else {
        Ok(mock_delete_card_hs(&*state.store, card_reference)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("card_delete_failure_message")?)
    }
}

///Mock api for local testing
pub async fn mock_call_to_locker_hs<'a>(
    db: &dyn db::StorageInterface,
    card_id: &str,
    payload: &payment_methods::StoreLockerReq<'a>,
    card_cvc: Option<String>,
    payment_method_id: Option<String>,
    customer_id: Option<&str>,
) -> errors::CustomResult<payment_methods::StoreCardResp, errors::VaultError> {
    let mut locker_mock_up = storage::LockerMockUpNew {
        card_id: card_id.to_string(),
        external_id: uuid::Uuid::new_v4().to_string(),
        card_fingerprint: uuid::Uuid::new_v4().to_string(),
        card_global_fingerprint: uuid::Uuid::new_v4().to_string(),
        merchant_id: "".to_string(),
        card_number: "4111111111111111".to_string(),
        card_exp_year: "2099".to_string(),
        card_exp_month: "12".to_string(),
        card_cvc,
        payment_method_id,
        customer_id: customer_id.map(str::to_string),
        name_on_card: None,
        nickname: None,
        enc_card_data: None,
    };
    locker_mock_up = match payload {
        payment_methods::StoreLockerReq::LockerCard(store_card_req) => storage::LockerMockUpNew {
            merchant_id: store_card_req.merchant_id.to_string(),
            card_number: store_card_req.card.card_number.peek().to_string(),
            card_exp_year: store_card_req.card.card_exp_year.peek().to_string(),
            card_exp_month: store_card_req.card.card_exp_month.peek().to_string(),
            name_on_card: store_card_req.card.name_on_card.to_owned().expose_option(),
            nickname: store_card_req.card.nick_name.to_owned(),
            ..locker_mock_up
        },
        payment_methods::StoreLockerReq::LockerGeneric(store_generic_req) => {
            storage::LockerMockUpNew {
                merchant_id: store_generic_req.merchant_id.to_string(),
                enc_card_data: Some(store_generic_req.enc_data.to_owned()),
                ..locker_mock_up
            }
        }
    };

    let response = db
        .insert_locker_mock_up(locker_mock_up)
        .await
        .change_context(errors::VaultError::SaveCardFailed)?;
    let payload = payment_methods::StoreCardRespPayload {
        card_reference: response.card_id,
        duplication_check: None,
    };
    Ok(payment_methods::StoreCardResp {
        status: "Ok".to_string(),
        error_code: None,
        error_message: None,
        payload: Some(payload),
    })
}

#[instrument(skip_all)]
pub async fn mock_get_card<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<(payment_methods::GetCardResponse, Option<String>), errors::VaultError> {
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    let add_card_response = payment_methods::AddCardResponse {
        card_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
        external_id: locker_mock_up.external_id,
        card_fingerprint: locker_mock_up.card_fingerprint.into(),
        card_global_fingerprint: locker_mock_up.card_global_fingerprint.into(),
        merchant_id: Some(locker_mock_up.merchant_id),
        card_number: cards::CardNumber::try_from(locker_mock_up.card_number)
            .change_context(errors::VaultError::ResponseDeserializationFailed)
            .attach_printable("Invalid card number format from the mock locker")
            .map(Some)?,
        card_exp_year: Some(locker_mock_up.card_exp_year.into()),
        card_exp_month: Some(locker_mock_up.card_exp_month.into()),
        name_on_card: locker_mock_up.name_on_card.map(|card| card.into()),
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
    };
    Ok((
        payment_methods::GetCardResponse {
            card: add_card_response,
        },
        locker_mock_up.card_cvc,
    ))
}

#[instrument(skip_all)]
pub async fn mock_get_payment_method<'a>(
    db: &dyn db::StorageInterface,
    key_store: &domain::MerchantKeyStore,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::GetPaymentMethodResponse, errors::VaultError> {
    let locker_mock_up = db
        .find_locker_by_card_id(card_id)
        .await
        .change_context(errors::VaultError::FetchPaymentMethodFailed)?;
    let dec_data = if let Some(e) = locker_mock_up.enc_card_data {
        decode_and_decrypt_locker_data(key_store, e).await
    } else {
        Err(report!(errors::VaultError::FetchPaymentMethodFailed))
    }?;
    let payment_method_response = payment_methods::AddPaymentMethodResponse {
        payment_method_id: locker_mock_up
            .payment_method_id
            .unwrap_or(locker_mock_up.card_id),
        external_id: locker_mock_up.external_id,
        merchant_id: Some(locker_mock_up.merchant_id),
        nickname: locker_mock_up.nickname,
        customer_id: locker_mock_up.customer_id,
        duplicate: locker_mock_up.duplicate,
        payment_method_data: dec_data,
    };
    Ok(payment_methods::GetPaymentMethodResponse {
        payment_method: payment_method_response,
    })
}

#[instrument(skip_all)]
pub async fn mock_delete_card_hs<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResp, errors::VaultError> {
    db.delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(payment_methods::DeleteCardResp {
        status: "Ok".to_string(),
        error_code: None,
        error_message: None,
    })
}

#[instrument(skip_all)]
pub async fn mock_delete_card<'a>(
    db: &dyn db::StorageInterface,
    card_id: &'a str,
) -> errors::CustomResult<payment_methods::DeleteCardResponse, errors::VaultError> {
    let locker_mock_up = db
        .delete_locker_mock_up(card_id)
        .await
        .change_context(errors::VaultError::FetchCardFailed)?;
    Ok(payment_methods::DeleteCardResponse {
        card_id: Some(locker_mock_up.card_id),
        external_id: Some(locker_mock_up.external_id),
        card_isin: None,
        status: "Ok".to_string(),
    })
}
//------------------------------------------------------------------------------
pub fn get_banks(
    state: &routes::AppState,
    pm_type: common_enums::enums::PaymentMethodType,
    connectors: Vec<String>,
) -> Result<Vec<BankCodeResponse>, errors::ApiErrorResponse> {
    let mut bank_names_hm: HashMap<String, HashSet<common_enums::enums::BankNames>> =
        HashMap::new();

    if matches!(
        pm_type,
        api_enums::PaymentMethodType::Giropay | api_enums::PaymentMethodType::Sofort
    ) {
        Ok(vec![BankCodeResponse {
            bank_name: vec![],
            eligible_connectors: connectors,
        }])
    } else {
        let mut bank_code_responses = vec![];
        for connector in &connectors {
            if let Some(connector_bank_names) = state.conf.bank_config.0.get(&pm_type) {
                if let Some(connector_hash_set) = connector_bank_names.0.get(connector) {
                    bank_names_hm.insert(connector.clone(), connector_hash_set.banks.clone());
                } else {
                    logger::error!("Could not find any configured connectors for payment_method -> {pm_type} for connector -> {connector}");
                }
            } else {
                logger::error!("Could not find any configured banks for payment_method -> {pm_type} for connector -> {connector}");
            }
        }

        let vector_of_hashsets = bank_names_hm
            .values()
            .map(|bank_names_hashset| bank_names_hashset.to_owned())
            .collect::<Vec<_>>();

        let mut common_bank_names = HashSet::new();
        if let Some(first_element) = vector_of_hashsets.first() {
            common_bank_names = vector_of_hashsets
                .iter()
                .skip(1)
                .fold(first_element.to_owned(), |acc, hs| {
                    acc.intersection(hs).cloned().collect()
                });
        }

        if !common_bank_names.is_empty() {
            bank_code_responses.push(BankCodeResponse {
                bank_name: common_bank_names.clone().into_iter().collect(),
                eligible_connectors: connectors.clone(),
            });
        }

        for connector in connectors {
            if let Some(all_bank_codes_for_connector) = bank_names_hm.get(&connector) {
                let remaining_bank_codes: HashSet<_> = all_bank_codes_for_connector
                    .difference(&common_bank_names)
                    .collect();

                if !remaining_bank_codes.is_empty() {
                    bank_code_responses.push(BankCodeResponse {
                        bank_name: remaining_bank_codes
                            .into_iter()
                            .map(|ele| ele.to_owned())
                            .collect(),
                        eligible_connectors: vec![connector],
                    })
                }
            } else {
                logger::error!("Could not find any configured banks for payment_method -> {pm_type} for connector -> {connector}");
            }
        }
        Ok(bank_code_responses)
    }
}

fn get_val(str: String, val: &serde_json::Value) -> Option<String> {
    str.split('.')
        .try_fold(val, |acc, x| acc.get(x))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub async fn list_payment_methods(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    mut req: api::PaymentMethodListRequest,
) -> errors::RouterResponse<api::PaymentMethodListResponse> {
    let db = &*state.store;
    let pm_config_mapping = &state.conf.pm_filters;

    let payment_intent = if let Some(cs) = &req.client_secret {
        if cs.starts_with("pm_") {
            validate_payment_method_and_client_secret(cs, db, &merchant_account).await?;
            None
        } else {
            helpers::verify_payment_intent_time_and_client_secret(
                db,
                &merchant_account,
                req.client_secret.clone(),
            )
            .await?
        }
    } else {
        None
    };

    let shipping_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                db,
                pi.shipping_address_id.clone(),
                &key_store,
                &pi.payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let billing_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                db,
                pi.billing_address_id.clone(),
                &key_store,
                &pi.payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    let customer = payment_intent
        .as_ref()
        .async_and_then(|pi| async {
            pi.customer_id
                .as_ref()
                .async_and_then(|cust| async {
                    db.find_customer_by_customer_id_merchant_id(
                        cust.as_str(),
                        &pi.merchant_id,
                        &key_store,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
                    .ok()
                })
                .await
        })
        .await;

    let payment_attempt = payment_intent
        .as_ref()
        .async_map(|pi| async {
            db.find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &pi.payment_id,
                &pi.merchant_id,
                &pi.active_attempt.get_id(),
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;
    let setup_future_usage = payment_intent.as_ref().and_then(|pi| pi.setup_future_usage);
    let payment_type = payment_attempt.as_ref().map(|pa| {
        let amount = api::Amount::from(pa.amount);
        let mandate_type = if pa.mandate_id.is_some() {
            Some(api::MandateTransactionType::RecurringMandateTransaction)
        } else if pa.mandate_details.is_some()
            || setup_future_usage
                .map(|future_usage| future_usage == common_enums::enums::FutureUsage::OffSession)
                .unwrap_or(false)
        {
            Some(api::MandateTransactionType::NewMandateTransaction)
        } else {
            None
        };

        helpers::infer_payment_type(&amount, mandate_type.as_ref())
    });

    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &merchant_account.merchant_id,
            false,
            &key_store,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let profile_id = payment_intent
        .as_ref()
        .async_map(|payment_intent| async {
            crate::core::utils::get_profile_id_from_business_details(
                payment_intent.business_country,
                payment_intent.business_label.as_ref(),
                &merchant_account,
                payment_intent.profile_id.as_ref(),
                db,
                false,
            )
            .await
            .attach_printable("Could not find profile id from business details")
        })
        .await
        .transpose()?;
    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        profile_id.as_ref(),
        &merchant_account.merchant_id,
    )
    .await?;

    // filter out connectors based on the business country
    let filtered_mcas =
        helpers::filter_mca_based_on_business_profile(all_mcas.clone(), profile_id.clone());

    logger::debug!(mca_before_filtering=?filtered_mcas);

    let mut response: Vec<ResponsePaymentMethodIntermediate> = vec![];

    // Key creation for storing PM_FILTER_CGRAPH
    #[cfg(feature = "business_profile_routing")]
    let key = {
        let profile_id = profile_id
            .clone()
            .get_required_value("profile_id")
            .change_context(errors::ApiErrorResponse::GenericNotFoundError {
                message: "Profile id not found".to_string(),
            })?;
        format!(
            "pm_filters_cgraph_{}_{}",
            &merchant_account.merchant_id, profile_id
        )
    };

    #[cfg(not(feature = "business_profile_routing"))]
    let key = { format!("pm_filters_cgraph_{}", &merchant_account.merchant_id) };

    if let Some(graph) = get_merchant_pm_filter_graph(&key).await {
        // Derivation of PM_FILTER_CGRAPH from MokaCache successful
        for mca in &filtered_mcas {
            let payment_methods = match &mca.payment_methods_enabled {
                Some(pm) => pm,
                None => continue,
            };
            filter_payment_methods(
                &graph,
                payment_methods,
                &mut req,
                &mut response,
                payment_intent.as_ref(),
                payment_attempt.as_ref(),
                billing_address.as_ref(),
                mca.connector_name.clone(),
                &state.conf.saved_payment_methods,
            )
            .await?;
        }
    } else {
        // No PM_FILTER_CGRAPH Cache present in MokaCache
        let mut builder = cgraph::ConstraintGraphBuilder::<'static, _>::new();
        for mca in &filtered_mcas {
            let payment_methods = match &mca.payment_methods_enabled {
                Some(pm) => pm,
                None => continue,
            };
            if let Err(e) = make_pm_graph(
                &mut builder,
                payment_methods,
                mca.connector_name.clone(),
                pm_config_mapping,
                &state.conf.mandates.supported_payment_methods,
                &state.conf.mandates.update_mandate_supported,
            ) {
                logger::error!(
                    "Failed to construct constraint graph for list payment methods {e:?}"
                );
            }
        }

        // Refreshing our CGraph cache
        let graph = refresh_pm_filters_cache(&key, builder.build()).await;

        for mca in &filtered_mcas {
            let payment_methods = match &mca.payment_methods_enabled {
                Some(pm) => pm,
                None => continue,
            };
            filter_payment_methods(
                &graph,
                payment_methods,
                &mut req,
                &mut response,
                payment_intent.as_ref(),
                payment_attempt.as_ref(),
                billing_address.as_ref(),
                mca.connector_name.clone(),
                &state.conf.saved_payment_methods,
            )
            .await?;
        }
    }

    // Filter out wallet payment method from mca if customer has already saved it
    customer
        .as_ref()
        .async_map(|customer| async {
            let wallet_pm_exists = response
                .iter()
                .any(|mca| mca.payment_method == enums::PaymentMethod::Wallet);
            if wallet_pm_exists {
                match db
                    .find_payment_method_by_customer_id_merchant_id_list(
                        &customer.customer_id,
                        &merchant_account.merchant_id,
                        None,
                    )
                    .await
                {
                    Ok(customer_payment_methods) => {
                        let customer_wallet_pm = customer_payment_methods
                            .iter()
                            .filter(|cust_pm| {
                                cust_pm.payment_method == Some(enums::PaymentMethod::Wallet)
                            })
                            .collect::<Vec<_>>();

                        response.retain(|mca| {
                            !(mca.payment_method == enums::PaymentMethod::Wallet
                                && customer_wallet_pm.iter().any(|cust_pm| {
                                    cust_pm.payment_method_type == Some(mca.payment_method_type)
                                }))
                        });
                        Ok(())
                    }
                    Err(error) => {
                        if error.current_context().is_db_not_found() {
                            Ok(())
                        } else {
                            Err(error)
                                .change_context(errors::ApiErrorResponse::InternalServerError)
                                .attach_printable("failed to find payment methods for a customer")
                        }
                    }
                }
            } else {
                Ok(())
            }
        })
        .await
        .transpose()?;

    let mut pmt_to_auth_connector = HashMap::new();

    if let Some((payment_attempt, payment_intent)) =
        payment_attempt.as_ref().zip(payment_intent.as_ref())
    {
        let routing_enabled_pms = HashSet::from([
            api_enums::PaymentMethod::BankTransfer,
            api_enums::PaymentMethod::BankDebit,
            api_enums::PaymentMethod::BankRedirect,
        ]);

        let routing_enabled_pm_types = HashSet::from([
            api_enums::PaymentMethodType::GooglePay,
            api_enums::PaymentMethodType::ApplePay,
            api_enums::PaymentMethodType::Klarna,
            api_enums::PaymentMethodType::Paypal,
        ]);

        let mut chosen = Vec::<api::SessionConnectorData>::new();
        for intermediate in &response {
            if routing_enabled_pm_types.contains(&intermediate.payment_method_type)
                || routing_enabled_pms.contains(&intermediate.payment_method)
            {
                let connector_data = api::ConnectorData::get_connector_by_name(
                    &state.clone().conf.connectors,
                    &intermediate.connector,
                    api::GetToken::from(intermediate.payment_method_type),
                    None,
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("invalid connector name received")?;

                chosen.push(api::SessionConnectorData {
                    payment_method_type: intermediate.payment_method_type,
                    connector: connector_data,
                    business_sub_label: None,
                });
            }
        }
        let sfr = SessionFlowRoutingInput {
            state: &state,
            country: shipping_address.clone().and_then(|ad| ad.country),
            key_store: &key_store,
            merchant_account: &merchant_account,
            payment_attempt,
            payment_intent,
            chosen,
        };
        let result = routing::perform_session_flow_routing(sfr, &enums::TransactionType::Payment)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("error performing session flow routing")?;

        response.retain(|intermediate| {
            if !routing_enabled_pm_types.contains(&intermediate.payment_method_type)
                && !routing_enabled_pms.contains(&intermediate.payment_method)
            {
                return true;
            }

            if let Some(choice) = result.get(&intermediate.payment_method_type) {
                intermediate.connector == choice.connector.connector_name.to_string()
            } else {
                false
            }
        });

        let mut routing_info: storage::PaymentRoutingInfo = payment_attempt
            .straight_through_algorithm
            .clone()
            .map(|val| val.parse_value("PaymentRoutingInfo"))
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Invalid PaymentRoutingInfo format found in payment attempt")?
            .unwrap_or_else(|| storage::PaymentRoutingInfo {
                algorithm: None,
                pre_routing_results: None,
            });

        let mut pre_routing_results: HashMap<
            api_enums::PaymentMethodType,
            routing_types::RoutableConnectorChoice,
        > = HashMap::new();

        for (pm_type, choice) in result {
            let routable_choice = routing_types::RoutableConnectorChoice {
                #[cfg(feature = "backwards_compatibility")]
                choice_kind: routing_types::RoutableChoiceKind::FullStruct,
                connector: choice
                    .connector
                    .connector_name
                    .to_string()
                    .parse::<api_enums::RoutableConnectors>()
                    .change_context(errors::ApiErrorResponse::InternalServerError)?,
                #[cfg(feature = "connector_choice_mca_id")]
                merchant_connector_id: choice.connector.merchant_connector_id,
                #[cfg(not(feature = "connector_choice_mca_id"))]
                sub_label: choice.sub_label,
            };

            pre_routing_results.insert(pm_type, routable_choice);
        }

        let redis_conn = db
            .get_redis_conn()
            .map_err(|redis_error| logger::error!(?redis_error))
            .ok();

        let mut val = Vec::new();

        for (payment_method_type, routable_connector_choice) in &pre_routing_results {
            #[cfg(not(feature = "connector_choice_mca_id"))]
            let connector_label = get_connector_label(
                payment_intent.business_country,
                payment_intent.business_label.as_ref(),
                #[cfg(not(feature = "connector_choice_mca_id"))]
                routable_connector_choice.sub_label.as_ref(),
                #[cfg(feature = "connector_choice_mca_id")]
                None,
                routable_connector_choice.connector.to_string().as_str(),
            );
            #[cfg(not(feature = "connector_choice_mca_id"))]
            let matched_mca = filtered_mcas
                .iter()
                .find(|m| connector_label == m.connector_label);

            #[cfg(feature = "connector_choice_mca_id")]
            let matched_mca = filtered_mcas.iter().find(|m| {
                routable_connector_choice.merchant_connector_id.as_ref()
                    == Some(&m.merchant_connector_id)
            });

            if let Some(m) = matched_mca {
                let pm_auth_config = m
                    .pm_auth_config
                    .as_ref()
                    .map(|config| {
                        serde_json::from_value::<PaymentMethodAuthConfig>(config.clone())
                            .change_context(errors::StorageError::DeserializationFailed)
                            .attach_printable("Failed to deserialize Payment Method Auth config")
                    })
                    .transpose()
                    .unwrap_or_else(|err| {
                        logger::error!(error=?err);
                        None
                    });

                let matched_config = match pm_auth_config {
                    Some(config) => {
                        let internal_config = config
                            .enabled_payment_methods
                            .iter()
                            .find(|config| config.payment_method_type == *payment_method_type)
                            .cloned();

                        internal_config
                    }
                    None => None,
                };

                if let Some(config) = matched_config {
                    pmt_to_auth_connector
                        .insert(*payment_method_type, config.connector_name.clone());
                    val.push(config);
                }
            }
        }

        let pm_auth_key = format!("pm_auth_{}", payment_intent.payment_id);
        let redis_expiry = state.conf.payment_method_auth.get_inner().redis_expiry;

        if let Some(rc) = redis_conn {
            rc.serialize_and_set_key_with_expiry(pm_auth_key.as_str(), val, redis_expiry)
                .await
                .attach_printable("Failed to store pm auth data in redis")
                .unwrap_or_else(|err| {
                    logger::error!(error=?err);
                })
        };

        routing_info.pre_routing_results = Some(pre_routing_results);

        let encoded = routing_info
            .encode_to_value()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to serialize payment routing info to value")?;

        let attempt_update = storage::PaymentAttemptUpdate::UpdateTrackers {
            payment_token: None,
            connector: None,
            straight_through_algorithm: Some(encoded),
            amount_capturable: None,
            updated_by: merchant_account.storage_scheme.to_string(),
            merchant_connector_id: None,
            surcharge_amount: None,
            tax_amount: None,
        };

        state
            .store
            .update_payment_attempt_with_attempt_id(
                payment_attempt.clone(),
                attempt_update,
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)?;
    }

    // Check for `use_billing_as_payment_method_billing` config under business_profile
    // If this is disabled, then the billing details in required fields will be empty and have to be collected by the customer
    let billing_address_for_calculating_required_fields = business_profile
        .as_ref()
        .and_then(|business_profile| business_profile.use_billing_as_payment_method_billing)
        .unwrap_or(true)
        .then_some(billing_address.as_ref())
        .flatten();

    let req = api_models::payments::PaymentsRequest::foreign_from((
        payment_attempt.as_ref(),
        shipping_address.as_ref(),
        billing_address_for_calculating_required_fields,
        customer.as_ref(),
    ));
    let req_val = serde_json::to_value(req).ok();
    logger::debug!(filtered_payment_methods=?response);

    let mut payment_experiences_consolidated_hm: HashMap<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<api_enums::PaymentExperience, Vec<String>>>,
    > = HashMap::new();

    let mut card_networks_consolidated_hm: HashMap<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<api_enums::CardNetwork, Vec<String>>>,
    > = HashMap::new();

    let mut banks_consolidated_hm: HashMap<api_enums::PaymentMethodType, Vec<String>> =
        HashMap::new();

    let mut bank_debits_consolidated_hm =
        HashMap::<api_enums::PaymentMethodType, Vec<String>>::new();

    let mut bank_transfer_consolidated_hm =
        HashMap::<api_enums::PaymentMethodType, Vec<String>>::new();

    // All the required fields will be stored here and later filtered out based on business profile config
    let mut required_fields_hm = HashMap::<
        api_enums::PaymentMethod,
        HashMap<api_enums::PaymentMethodType, HashMap<String, RequiredFieldInfo>>,
    >::new();

    for element in response.clone() {
        let payment_method = element.payment_method;
        let payment_method_type = element.payment_method_type;
        let connector = element.connector.clone();

        let connector_variant = api_enums::Connector::from_str(connector.as_str())
            .change_context(errors::ConnectorError::InvalidConnectorName)
            .change_context(errors::ApiErrorResponse::InvalidDataValue {
                field_name: "connector",
            })
            .attach_printable_lazy(|| format!("unable to parse connector name {connector:?}"))?;
        state.conf.required_fields.0.get(&payment_method).map(
            |required_fields_hm_for_each_payment_method_type| {
                required_fields_hm_for_each_payment_method_type
                    .0
                    .get(&payment_method_type)
                    .map(|required_fields_hm_for_each_connector| {
                        required_fields_hm.entry(payment_method).or_default();
                        required_fields_hm_for_each_connector
                            .fields
                            .get(&connector_variant)
                            .map(|required_fields_final| {
                                let mut required_fields_hs = required_fields_final.common.clone();
                                if let Some(pa) = payment_attempt.as_ref() {
                                    if let Some(_mandate) = &pa.mandate_details {
                                        required_fields_hs
                                            .extend(required_fields_final.mandate.clone());
                                    } else {
                                        required_fields_hs
                                            .extend(required_fields_final.non_mandate.clone());
                                    }
                                }

                                let should_send_shipping_details =
                                    business_profile.clone().and_then(|business_profile| {
                                        business_profile
                                            .collect_shipping_details_from_wallet_connector
                                    });

                                // Remove shipping fields from required fields based on business profile configuration
                                if should_send_shipping_details != Some(true) {
                                    let shipping_variants =
                                        api_enums::FieldType::get_shipping_variants();

                                    let keys_to_be_removed = required_fields_hs
                                        .iter()
                                        .filter(|(_key, value)| {
                                            shipping_variants.contains(&value.field_type)
                                        })
                                        .map(|(key, _value)| key.to_string())
                                        .collect::<Vec<_>>();

                                    keys_to_be_removed.iter().for_each(|key_to_be_removed| {
                                        required_fields_hs.remove(key_to_be_removed);
                                    });
                                }

                                // get the config, check the enums while adding
                                {
                                    for (key, val) in &mut required_fields_hs {
                                        let temp = req_val
                                            .as_ref()
                                            .and_then(|r| get_val(key.to_owned(), r));
                                        if let Some(s) = temp {
                                            val.value = Some(s.into())
                                        };
                                    }
                                }

                                let existing_req_fields_hs = required_fields_hm
                                    .get_mut(&payment_method)
                                    .and_then(|inner_hm| inner_hm.get_mut(&payment_method_type));

                                // If payment_method_type already exist in required_fields_hm, extend the required_fields hs to existing hs.
                                if let Some(inner_hs) = existing_req_fields_hs {
                                    inner_hs.extend(required_fields_hs);
                                } else {
                                    required_fields_hm.get_mut(&payment_method).map(|inner_hm| {
                                        inner_hm.insert(payment_method_type, required_fields_hs)
                                    });
                                }
                            })
                    })
            },
        );

        if let Some(payment_experience) = element.payment_experience {
            if let Some(payment_method_hm) =
                payment_experiences_consolidated_hm.get_mut(&payment_method)
            {
                if let Some(payment_method_type_hm) =
                    payment_method_hm.get_mut(&payment_method_type)
                {
                    if let Some(vector_of_connectors) =
                        payment_method_type_hm.get_mut(&payment_experience)
                    {
                        vector_of_connectors.push(connector);
                    } else {
                        payment_method_type_hm.insert(payment_experience, vec![connector]);
                    }
                } else {
                    payment_method_hm.insert(
                        payment_method_type,
                        HashMap::from([(payment_experience, vec![connector])]),
                    );
                }
            } else {
                let inner_hm = HashMap::from([(payment_experience, vec![connector])]);
                let payment_method_type_hm = HashMap::from([(payment_method_type, inner_hm)]);
                payment_experiences_consolidated_hm.insert(payment_method, payment_method_type_hm);
            }
        }

        if let Some(card_networks) = element.card_networks {
            if let Some(payment_method_hm) = card_networks_consolidated_hm.get_mut(&payment_method)
            {
                if let Some(payment_method_type_hm) =
                    payment_method_hm.get_mut(&payment_method_type)
                {
                    for card_network in card_networks {
                        if let Some(vector_of_connectors) =
                            payment_method_type_hm.get_mut(&card_network)
                        {
                            let connector = element.connector.clone();
                            vector_of_connectors.push(connector);
                        } else {
                            let connector = element.connector.clone();
                            payment_method_type_hm.insert(card_network, vec![connector]);
                        }
                    }
                } else {
                    let mut inner_hashmap: HashMap<api_enums::CardNetwork, Vec<String>> =
                        HashMap::new();
                    for card_network in card_networks {
                        if let Some(vector_of_connectors) = inner_hashmap.get_mut(&card_network) {
                            let connector = element.connector.clone();
                            vector_of_connectors.push(connector);
                        } else {
                            let connector = element.connector.clone();
                            inner_hashmap.insert(card_network, vec![connector]);
                        }
                    }
                    payment_method_hm.insert(payment_method_type, inner_hashmap);
                }
            } else {
                let mut inner_hashmap: HashMap<api_enums::CardNetwork, Vec<String>> =
                    HashMap::new();
                for card_network in card_networks {
                    if let Some(vector_of_connectors) = inner_hashmap.get_mut(&card_network) {
                        let connector = element.connector.clone();
                        vector_of_connectors.push(connector);
                    } else {
                        let connector = element.connector.clone();
                        inner_hashmap.insert(card_network, vec![connector]);
                    }
                }
                let payment_method_type_hm = HashMap::from([(payment_method_type, inner_hashmap)]);
                card_networks_consolidated_hm.insert(payment_method, payment_method_type_hm);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankRedirect {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                banks_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                banks_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankDebit {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                bank_debits_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                bank_debits_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }

        if element.payment_method == api_enums::PaymentMethod::BankTransfer {
            let connector = element.connector.clone();
            if let Some(vector_of_connectors) =
                bank_transfer_consolidated_hm.get_mut(&element.payment_method_type)
            {
                vector_of_connectors.push(connector);
            } else {
                bank_transfer_consolidated_hm.insert(element.payment_method_type, vec![connector]);
            }
        }
    }

    let mut payment_method_responses: Vec<ResponsePaymentMethodsEnabled> = vec![];
    for key in payment_experiences_consolidated_hm.iter() {
        let mut payment_method_types = vec![];
        for payment_method_types_hm in key.1 {
            let mut payment_experience_types = vec![];
            for payment_experience_type in payment_method_types_hm.1 {
                payment_experience_types.push(PaymentExperienceTypes {
                    payment_experience_type: *payment_experience_type.0,
                    eligible_connectors: payment_experience_type.1.clone(),
                })
            }

            payment_method_types.push(ResponsePaymentMethodTypes {
                payment_method_type: *payment_method_types_hm.0,
                payment_experience: Some(payment_experience_types),
                card_networks: None,
                bank_names: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for PayLater payment method
                required_fields: required_fields_hm
                    .get(key.0)
                    .and_then(|inner_hm| inner_hm.get(payment_method_types_hm.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector
                    .get(payment_method_types_hm.0)
                    .cloned(),
            })
        }

        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: *key.0,
            payment_method_types,
        })
    }

    for key in card_networks_consolidated_hm.iter() {
        let mut payment_method_types = vec![];
        for payment_method_types_hm in key.1 {
            let mut card_network_types = vec![];
            for card_network_type in payment_method_types_hm.1 {
                card_network_types.push(CardNetworkTypes {
                    card_network: card_network_type.0.clone(),
                    eligible_connectors: card_network_type.1.clone(),
                    surcharge_details: None,
                })
            }

            payment_method_types.push(ResponsePaymentMethodTypes {
                payment_method_type: *payment_method_types_hm.0,
                card_networks: Some(card_network_types),
                payment_experience: None,
                bank_names: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for Card payment method
                required_fields: required_fields_hm
                    .get(key.0)
                    .and_then(|inner_hm| inner_hm.get(payment_method_types_hm.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector
                    .get(payment_method_types_hm.0)
                    .cloned(),
            })
        }

        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: *key.0,
            payment_method_types,
        })
    }

    let mut bank_redirect_payment_method_types = vec![];

    for key in banks_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        let bank_names = get_banks(&state, payment_method_type, connectors)?;
        bank_redirect_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: Some(bank_names),
                payment_experience: None,
                card_networks: None,
                bank_debits: None,
                bank_transfers: None,
                // Required fields for BankRedirect payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankRedirect)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector.get(&payment_method_type).cloned(),
            }
        })
    }

    if !bank_redirect_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankRedirect,
            payment_method_types: bank_redirect_payment_method_types,
        });
    }

    let mut bank_debit_payment_method_types = vec![];

    for key in bank_debits_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        bank_debit_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: None,
                payment_experience: None,
                card_networks: None,
                bank_debits: Some(api_models::payment_methods::BankDebitTypes {
                    eligible_connectors: connectors.clone(),
                }),
                bank_transfers: None,
                // Required fields for BankDebit payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankDebit)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector.get(&payment_method_type).cloned(),
            }
        })
    }

    if !bank_debit_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankDebit,
            payment_method_types: bank_debit_payment_method_types,
        });
    }

    let mut bank_transfer_payment_method_types = vec![];

    for key in bank_transfer_consolidated_hm.iter() {
        let payment_method_type = *key.0;
        let connectors = key.1.clone();
        bank_transfer_payment_method_types.push({
            ResponsePaymentMethodTypes {
                payment_method_type,
                bank_names: None,
                payment_experience: None,
                card_networks: None,
                bank_debits: None,
                bank_transfers: Some(api_models::payment_methods::BankTransferTypes {
                    eligible_connectors: connectors,
                }),
                // Required fields for BankTransfer payment method
                required_fields: required_fields_hm
                    .get(&api_enums::PaymentMethod::BankTransfer)
                    .and_then(|inner_hm| inner_hm.get(key.0))
                    .cloned(),
                surcharge_details: None,
                pm_auth_connector: pmt_to_auth_connector.get(&payment_method_type).cloned(),
            }
        })
    }

    if !bank_transfer_payment_method_types.is_empty() {
        payment_method_responses.push(ResponsePaymentMethodsEnabled {
            payment_method: api_enums::PaymentMethod::BankTransfer,
            payment_method_types: bank_transfer_payment_method_types,
        });
    }
    let currency = payment_intent.as_ref().and_then(|pi| pi.currency);
    let merchant_surcharge_configs =
        if let Some((payment_attempt, payment_intent, business_profile)) = payment_attempt
            .as_ref()
            .zip(payment_intent)
            .zip(business_profile)
            .map(|((pa, pi), bp)| (pa, pi, bp))
        {
            Box::pin(call_surcharge_decision_management(
                state,
                &merchant_account,
                &business_profile,
                payment_attempt,
                payment_intent,
                billing_address,
                &mut payment_method_responses,
            ))
            .await?
        } else {
            api_surcharge_decision_configs::MerchantSurchargeConfigs::default()
        };
    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodListResponse {
            redirect_url: merchant_account.return_url,
            merchant_name: merchant_account.merchant_name,
            payment_type,
            payment_methods: payment_method_responses,
            mandate_payment: payment_attempt.and_then(|inner| inner.mandate_details).map(
                |d| match d {
                    hyperswitch_domain_models::mandates::MandateDataType::SingleUse(i) => {
                        api::MandateType::SingleUse(api::MandateAmountData {
                            amount: i.amount,
                            currency: i.currency,
                            start_date: i.start_date,
                            end_date: i.end_date,
                            metadata: i.metadata,
                        })
                    }
                    hyperswitch_domain_models::mandates::MandateDataType::MultiUse(Some(i)) => {
                        api::MandateType::MultiUse(Some(api::MandateAmountData {
                            amount: i.amount,
                            currency: i.currency,
                            start_date: i.start_date,
                            end_date: i.end_date,
                            metadata: i.metadata,
                        }))
                    }
                    hyperswitch_domain_models::mandates::MandateDataType::MultiUse(None) => {
                        api::MandateType::MultiUse(None)
                    }
                },
            ),
            show_surcharge_breakup_screen: merchant_surcharge_configs
                .show_surcharge_breakup_screen
                .unwrap_or_default(),
            currency,
        },
    ))
}

async fn validate_payment_method_and_client_secret(
    cs: &String,
    db: &dyn db::StorageInterface,
    merchant_account: &domain::MerchantAccount,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let pm_vec = cs.split("_secret").collect::<Vec<&str>>();
    let pm_id = pm_vec
        .first()
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })?;

    let payment_method = db
        .find_payment_method(pm_id, merchant_account.storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    let client_secret_expired =
        authenticate_pm_client_secret_and_check_expiry(cs, &payment_method)?;
    if client_secret_expired {
        return Err::<(), error_stack::Report<errors::ApiErrorResponse>>(
            (errors::ApiErrorResponse::ClientSecretExpired).into(),
        );
    }
    Ok(())
}

pub async fn call_surcharge_decision_management(
    state: routes::AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &BusinessProfile,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: storage::PaymentIntent,
    billing_address: Option<domain::Address>,
    response_payment_method_types: &mut [ResponsePaymentMethodsEnabled],
) -> errors::RouterResult<api_surcharge_decision_configs::MerchantSurchargeConfigs> {
    let algorithm_ref: routing_types::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();
    let (surcharge_results, merchant_sucharge_configs) =
        perform_surcharge_decision_management_for_payment_method_list(
            &state,
            algorithm_ref,
            payment_attempt,
            &payment_intent,
            billing_address.as_ref().map(Into::into),
            response_payment_method_types,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error performing surcharge decision operation")?;
    if !surcharge_results.is_empty_result() {
        surcharge_results
            .persist_individual_surcharge_details_in_redis(&state, business_profile)
            .await?;
        let _ = state
            .store
            .update_payment_intent(
                payment_intent,
                storage::PaymentIntentUpdate::SurchargeApplicableUpdate {
                    surcharge_applicable: true,
                    updated_by: merchant_account.storage_scheme.to_string(),
                },
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to update surcharge_applicable in Payment Intent");
    }
    Ok(merchant_sucharge_configs)
}

pub async fn call_surcharge_decision_management_for_saved_card(
    state: &routes::AppState,
    merchant_account: &domain::MerchantAccount,
    business_profile: &BusinessProfile,
    payment_attempt: &storage::PaymentAttempt,
    payment_intent: storage::PaymentIntent,
    customer_payment_method_response: &mut api::CustomerPaymentMethodsListResponse,
) -> errors::RouterResult<()> {
    let algorithm_ref: routing_types::RoutingAlgorithmRef = merchant_account
        .routing_algorithm
        .clone()
        .map(|val| val.parse_value("routing algorithm"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Could not decode the routing algorithm")?
        .unwrap_or_default();
    let surcharge_results = perform_surcharge_decision_management_for_saved_cards(
        state,
        algorithm_ref,
        payment_attempt,
        &payment_intent,
        &mut customer_payment_method_response.customer_payment_methods,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("error performing surcharge decision operation")?;
    if !surcharge_results.is_empty_result() {
        surcharge_results
            .persist_individual_surcharge_details_in_redis(state, business_profile)
            .await?;
        let _ = state
            .store
            .update_payment_intent(
                payment_intent,
                storage::PaymentIntentUpdate::SurchargeApplicableUpdate {
                    surcharge_applicable: true,
                    updated_by: merchant_account.storage_scheme.to_string(),
                },
                merchant_account.storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
            .attach_printable("Failed to update surcharge_applicable in Payment Intent");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn filter_payment_methods(
    graph: &ConstraintGraph<'_, dir::DirValue>,
    payment_methods: &[serde_json::Value],
    req: &mut api::PaymentMethodListRequest,
    resp: &mut Vec<ResponsePaymentMethodIntermediate>,
    payment_intent: Option<&storage::PaymentIntent>,
    payment_attempt: Option<&storage::PaymentAttempt>,
    address: Option<&domain::Address>,
    connector: String,
    saved_payment_methods: &settings::EligiblePaymentMethods,
) -> errors::CustomResult<(), errors::ApiErrorResponse> {
    for payment_method in payment_methods.iter() {
        let parse_result = serde_json::from_value::<PaymentMethodsEnabled>(payment_method.clone());
        if let Ok(payment_methods_enabled) = parse_result {
            let payment_method = payment_methods_enabled.payment_method;

            let allowed_payment_method_types = payment_intent
                .and_then(|payment_intent| {
                    payment_intent
                        .allowed_payment_method_types
                        .clone()
                        .map(|val| val.parse_value("Vec<PaymentMethodType>"))
                        .transpose()
                        .unwrap_or_else(|error| {
                            logger::error!(%error, "Failed to deserialize PaymentIntent allowed_payment_method_types"); None
                        })
                });

            for payment_method_type_info in payment_methods_enabled
                .payment_method_types
                .unwrap_or_default()
            {
                if filter_recurring_based(&payment_method_type_info, req.recurring_enabled)
                    && filter_installment_based(
                        &payment_method_type_info,
                        req.installment_payment_enabled,
                    )
                    && filter_amount_based(
                        &payment_method_type_info,
                        req.amount
                            .map(|minor_amount| minor_amount.get_amount_as_i64()),
                    )
                {
                    let payment_method_object = payment_method_type_info.clone();

                    let pm_dir_value: dir::DirValue =
                        (payment_method_type_info.payment_method_type, payment_method)
                            .into_dir_value()
                            .change_context(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("pm_value_node not created")?;

                    let connector_variant = api_enums::Connector::from_str(connector.as_str())
                        .change_context(errors::ConnectorError::InvalidConnectorName)
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "connector",
                        })
                        .attach_printable_lazy(|| {
                            format!("unable to parse connector name {connector:?}")
                        })?;

                    let mut context_values: Vec<dir::DirValue> = Vec::new();
                    context_values.push(pm_dir_value.clone());

                    payment_intent.map(|intent| {
                        intent.currency.map(|currency| {
                            context_values.push(dir::DirValue::PaymentCurrency(currency))
                        })
                    });
                    address.map(|address| {
                        address.country.map(|country| {
                            context_values.push(dir::DirValue::BillingCountry(
                                common_enums::Country::from_alpha2(country),
                            ))
                        })
                    });

                    let filter_pm_based_on_allowed_types = filter_pm_based_on_allowed_types(
                        allowed_payment_method_types.as_ref(),
                        &payment_method_object.payment_method_type,
                    );

                    if payment_attempt
                        .and_then(|attempt| attempt.mandate_details.as_ref())
                        .is_some()
                    {
                        context_values.push(dir::DirValue::PaymentType(
                            euclid::enums::PaymentType::NewMandate,
                        ));
                        if let Ok(connector) = api_enums::RoutableConnectors::from_str(
                            connector_variant.to_string().as_str(),
                        ) {
                            context_values.push(dir::DirValue::Connector(Box::new(
                                api_models::routing::ast::ConnectorChoice {
                                    connector,
                                    #[cfg(not(feature = "connector_choice_mca_id"))]
                                    sub_label: None,
                                },
                            )));
                        };
                    };

                    payment_attempt
                        .and_then(|attempt| attempt.mandate_data.as_ref())
                        .map(|mandate_detail| {
                            if mandate_detail.update_mandate_id.is_some() {
                                context_values.push(dir::DirValue::PaymentType(
                                    euclid::enums::PaymentType::UpdateMandate,
                                ));
                                if let Ok(connector) = api_enums::RoutableConnectors::from_str(
                                    connector_variant.to_string().as_str(),
                                ) {
                                    context_values.push(dir::DirValue::Connector(Box::new(
                                        api_models::routing::ast::ConnectorChoice {
                                            connector,
                                            #[cfg(not(feature = "connector_choice_mca_id"))]
                                            sub_label: None,
                                        },
                                    )));
                                };
                            }
                        });

                    payment_attempt
                        .map(|attempt| {
                            attempt.mandate_data.is_none() && attempt.mandate_details.is_none()
                        })
                        .and_then(|res| {
                            res.then(|| {
                                context_values.push(dir::DirValue::PaymentType(
                                    euclid::enums::PaymentType::NonMandate,
                                ))
                            })
                        });

                    payment_attempt
                        .and_then(|inner| inner.capture_method)
                        .and_then(|capture_method| {
                            (capture_method == common_enums::CaptureMethod::Manual).then(|| {
                                context_values.push(dir::DirValue::CaptureMethod(
                                    common_enums::CaptureMethod::Manual,
                                ));
                            })
                        });

                    let filter_pm_card_network_based = filter_pm_card_network_based(
                        payment_method_object.card_networks.as_ref(),
                        req.card_networks.as_ref(),
                        &payment_method_object.payment_method_type,
                    );

                    let saved_payment_methods_filter = req
                        .client_secret
                        .as_ref()
                        .map(|cs| {
                            if cs.starts_with("pm_") {
                                saved_payment_methods
                                    .sdk_eligible_payment_methods
                                    .contains(payment_method.to_string().as_str())
                            } else {
                                true
                            }
                        })
                        .unwrap_or(true);

                    let context = AnalysisContext::from_dir_values(context_values.clone());

                    let result = graph.key_value_analysis(
                        pm_dir_value.clone(),
                        &context,
                        &mut cgraph::Memoization::new(),
                        &mut cgraph::CycleCheck::new(),
                        None,
                    );
                    if filter_pm_based_on_allowed_types
                        && filter_pm_card_network_based
                        && saved_payment_methods_filter
                        && matches!(result, Ok(()))
                    {
                        let response_pm_type = ResponsePaymentMethodIntermediate::new(
                            payment_method_object,
                            connector.clone(),
                            payment_method,
                        );
                        resp.push(response_pm_type);
                    } else {
                        logger::error!("Filtering Payment Methods Failed");
                    }
                }
            }
        }
    }
    Ok(())
}

fn filter_pm_based_on_allowed_types(
    allowed_types: Option<&Vec<api_enums::PaymentMethodType>>,
    payment_method_type: &api_enums::PaymentMethodType,
) -> bool {
    allowed_types.map_or(true, |pm| pm.contains(payment_method_type))
}

fn filter_amount_based(payment_method: &RequestPaymentMethodTypes, amount: Option<i64>) -> bool {
    let min_check = amount
        .and_then(|amt| {
            payment_method
                .minimum_amount
                .map(|min_amt| amt >= min_amt.into())
        })
        .unwrap_or(true);
    let max_check = amount
        .and_then(|amt| {
            payment_method
                .maximum_amount
                .map(|max_amt| amt <= max_amt.into())
        })
        .unwrap_or(true);
    (min_check && max_check) || amount == Some(0)
}

fn filter_recurring_based(
    payment_method: &RequestPaymentMethodTypes,
    recurring_enabled: Option<bool>,
) -> bool {
    recurring_enabled.map_or(true, |enabled| payment_method.recurring_enabled == enabled)
}

fn filter_installment_based(
    payment_method: &RequestPaymentMethodTypes,
    installment_payment_enabled: Option<bool>,
) -> bool {
    installment_payment_enabled.map_or(true, |enabled| {
        payment_method.installment_payment_enabled == enabled
    })
}

fn filter_pm_card_network_based(
    pm_card_networks: Option<&Vec<api_enums::CardNetwork>>,
    request_card_networks: Option<&Vec<api_enums::CardNetwork>>,
    pm_type: &api_enums::PaymentMethodType,
) -> bool {
    match pm_type {
        api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
            match (pm_card_networks, request_card_networks) {
                (Some(pm_card_networks), Some(request_card_networks)) => request_card_networks
                    .iter()
                    .all(|card_network| pm_card_networks.contains(card_network)),
                (None, Some(_)) => false,
                _ => true,
            }
        }
        _ => true,
    }
}

pub async fn do_list_customer_pm_fetch_customer_if_not_passed(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: Option<api::PaymentMethodListRequest>,
    customer_id: Option<&str>,
) -> errors::RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let db = state.store.as_ref();
    let limit = req.clone().and_then(|pml_req| pml_req.limit);

    if let Some(customer_id) = customer_id {
        Box::pin(list_customer_payment_method(
            &state,
            merchant_account,
            key_store,
            None,
            customer_id,
            limit,
        ))
        .await
    } else {
        let cloned_secret = req.and_then(|r| r.client_secret.as_ref().cloned());
        let payment_intent: Option<hyperswitch_domain_models::payments::PaymentIntent> =
            helpers::verify_payment_intent_time_and_client_secret(
                db,
                &merchant_account,
                cloned_secret,
            )
            .await?;

        match payment_intent
            .as_ref()
            .and_then(|intent| intent.customer_id.to_owned())
        {
            Some(customer_id) => {
                Box::pin(list_customer_payment_method(
                    &state,
                    merchant_account,
                    key_store,
                    payment_intent,
                    &customer_id,
                    limit,
                ))
                .await
            }
            None => {
                let response = api::CustomerPaymentMethodsListResponse {
                    customer_payment_methods: Vec::new(),
                    is_guest_customer: Some(true),
                };
                Ok(services::ApplicationResponse::Json(response))
            }
        }
    }
}

pub async fn list_customer_payment_method(
    state: &routes::AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payment_intent: Option<storage::PaymentIntent>,
    customer_id: &str,
    limit: Option<i64>,
) -> errors::RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let db = &*state.store;
    let off_session_payment_flag = payment_intent
        .as_ref()
        .map(|pi| {
            matches!(
                pi.setup_future_usage,
                Some(common_enums::FutureUsage::OffSession)
            )
        })
        .unwrap_or(false);

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            customer_id,
            &merchant_account.merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let key = key_store.key.get_inner().peek();

    let is_requires_cvv = db
        .find_config_by_key_unwrap_or(
            format!("{}_requires_cvv", merchant_account.merchant_id).as_str(),
            Some("true".to_string()),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch requires_cvv config")?;

    let requires_cvv = is_requires_cvv.config != "false";

    let resp = db
        .find_payment_method_by_customer_id_merchant_id_status(
            customer_id,
            &merchant_account.merchant_id,
            common_enums::PaymentMethodStatus::Active,
            limit,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    //let mca = query::find_mca_by_merchant_id(conn, &merchant_account.merchant_id)?;
    let mut customer_pms = Vec::new();
    for pm in resp.into_iter() {
        let parent_payment_method_token = generate_id(consts::ID_LENGTH, "token");

        let payment_method = pm.payment_method.get_required_value("payment_method")?;

        let payment_method_retrieval_context = match payment_method {
            enums::PaymentMethod::Card => {
                let card_details = get_card_details_with_locker_fallback(&pm, key, state).await?;

                if card_details.is_some() {
                    PaymentMethodListContext {
                        card_details,
                        #[cfg(feature = "payouts")]
                        bank_transfer_details: None,
                        hyperswitch_token_data: PaymentTokenData::permanent_card(
                            Some(pm.payment_method_id.clone()),
                            pm.locker_id.clone().or(Some(pm.payment_method_id.clone())),
                            pm.locker_id.clone().unwrap_or(pm.payment_method_id.clone()),
                        ),
                    }
                } else {
                    continue;
                }
            }

            enums::PaymentMethod::BankDebit => {
                // Retrieve the pm_auth connector details so that it can be tokenized
                let bank_account_token_data = get_bank_account_connector_details(&pm, key)
                    .await
                    .unwrap_or_else(|err| {
                        logger::error!(error=?err);
                        None
                    });
                if let Some(data) = bank_account_token_data {
                    let token_data = PaymentTokenData::AuthBankDebit(data);

                    PaymentMethodListContext {
                        card_details: None,
                        #[cfg(feature = "payouts")]
                        bank_transfer_details: None,
                        hyperswitch_token_data: token_data,
                    }
                } else {
                    continue;
                }
            }

            enums::PaymentMethod::Wallet => PaymentMethodListContext {
                card_details: None,
                #[cfg(feature = "payouts")]
                bank_transfer_details: None,
                hyperswitch_token_data: PaymentTokenData::wallet_token(
                    pm.payment_method_id.clone(),
                ),
            },

            #[cfg(feature = "payouts")]
            enums::PaymentMethod::BankTransfer => PaymentMethodListContext {
                card_details: None,
                bank_transfer_details: Some(
                    get_bank_from_hs_locker(
                        state,
                        &key_store,
                        &parent_payment_method_token,
                        &pm.customer_id,
                        &pm.merchant_id,
                        pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
                    )
                    .await?,
                ),
                hyperswitch_token_data: PaymentTokenData::temporary_generic(
                    parent_payment_method_token.clone(),
                ),
            },

            _ => PaymentMethodListContext {
                card_details: None,
                #[cfg(feature = "payouts")]
                bank_transfer_details: None,
                hyperswitch_token_data: PaymentTokenData::temporary_generic(generate_id(
                    consts::ID_LENGTH,
                    "token",
                )),
            },
        };

        // Retrieve the masked bank details to be sent as a response
        let bank_details = if payment_method == enums::PaymentMethod::BankDebit {
            get_masked_bank_details(&pm, key)
                .await
                .unwrap_or_else(|err| {
                    logger::error!(error=?err);
                    None
                })
        } else {
            None
        };

        let payment_method_billing = decrypt_generic_data::<api_models::payments::Address>(
            pm.payment_method_billing_address,
            key,
        )
        .await
        .attach_printable("unable to decrypt payment method billing address details")?;
        let connector_mandate_details = pm
            .connector_mandate_details
            .clone()
            .map(|val| {
                val.parse_value::<storage::PaymentsMandateReference>("PaymentsMandateReference")
            })
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to deserialize to Payment Mandate Reference ")?;
        let mca_enabled = get_mca_status(
            state,
            &key_store,
            &merchant_account.merchant_id,
            connector_mandate_details,
        )
        .await?;
        // Need validation for enabled payment method ,querying MCA
        let pma = api::CustomerPaymentMethod {
            payment_token: parent_payment_method_token.to_owned(),
            payment_method_id: pm.payment_method_id.clone(),
            customer_id: pm.customer_id,
            payment_method,
            payment_method_type: pm.payment_method_type,
            payment_method_issuer: pm.payment_method_issuer,
            card: payment_method_retrieval_context.card_details,
            metadata: pm.metadata,
            payment_method_issuer_code: pm.payment_method_issuer_code,
            recurring_enabled: mca_enabled,
            installment_payment_enabled: false,
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            created: Some(pm.created_at),
            #[cfg(feature = "payouts")]
            bank_transfer: payment_method_retrieval_context.bank_transfer_details,
            bank: bank_details,
            surcharge_details: None,
            requires_cvv: requires_cvv
                && !(off_session_payment_flag && pm.connector_mandate_details.is_some()),
            last_used_at: Some(pm.last_used_at),
            default_payment_method_set: customer.default_payment_method_id.is_some()
                && customer.default_payment_method_id == Some(pm.payment_method_id),
            billing: payment_method_billing,
        };
        customer_pms.push(pma.to_owned());

        let intent_created = payment_intent.as_ref().map(|intent| intent.created_at);

        let redis_conn = state
            .store
            .get_redis_conn()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to get redis connection")?;
        ParentPaymentMethodToken::create_key_for_token((
            &parent_payment_method_token,
            pma.payment_method,
        ))
        .insert(
            intent_created,
            payment_method_retrieval_context.hyperswitch_token_data,
            state,
        )
        .await?;

        if let Some(metadata) = pma.metadata {
            let pm_metadata_vec: payment_methods::PaymentMethodMetadata = metadata
                .parse_value("PaymentMethodMetadata")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable(
                    "Failed to deserialize metadata to PaymentmethodMetadata struct",
                )?;

            for pm_metadata in pm_metadata_vec.payment_method_tokenization {
                let key = format!(
                    "pm_token_{}_{}_{}",
                    parent_payment_method_token, pma.payment_method, pm_metadata.0
                );
                let current_datetime_utc = common_utils::date_time::now();
                let time_elapsed = current_datetime_utc
                    - payment_intent
                        .as_ref()
                        .map(|intent| intent.created_at)
                        .unwrap_or_else(|| current_datetime_utc);
                redis_conn
                    .set_key_with_expiry(
                        &key,
                        pm_metadata.1,
                        consts::TOKEN_TTL - time_elapsed.whole_seconds(),
                    )
                    .await
                    .change_context(errors::StorageError::KVError)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to add data in redis")?;
            }
        }
    }

    let mut response = api::CustomerPaymentMethodsListResponse {
        customer_payment_methods: customer_pms,
        is_guest_customer: payment_intent.as_ref().map(|_| false), //to return this key only when the request is tied to a payment intent
    };
    let payment_attempt = payment_intent
        .as_ref()
        .async_map(|payment_intent| async {
            state
                .store
                .find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                    &payment_intent.payment_id,
                    &merchant_account.merchant_id,
                    &payment_intent.active_attempt.get_id(),
                    merchant_account.storage_scheme,
                )
                .await
                .to_not_found_response(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;

    let profile_id = payment_intent
        .as_ref()
        .async_map(|payment_intent| async {
            crate::core::utils::get_profile_id_from_business_details(
                payment_intent.business_country,
                payment_intent.business_label.as_ref(),
                &merchant_account,
                payment_intent.profile_id.as_ref(),
                db,
                false,
            )
            .await
            .attach_printable("Could not find profile id from business details")
        })
        .await
        .transpose()?;
    let business_profile = core_utils::validate_and_get_business_profile(
        db,
        profile_id.as_ref(),
        &merchant_account.merchant_id,
    )
    .await?;

    if let Some((payment_attempt, payment_intent, business_profile)) = payment_attempt
        .zip(payment_intent)
        .zip(business_profile)
        .map(|((pa, pi), bp)| (pa, pi, bp))
    {
        call_surcharge_decision_management_for_saved_card(
            state,
            &merchant_account,
            &business_profile,
            &payment_attempt,
            payment_intent,
            &mut response,
        )
        .await?;
    }

    Ok(services::ApplicationResponse::Json(response))
}
pub async fn get_mca_status(
    state: &routes::AppState,
    key_store: &domain::MerchantKeyStore,
    merchant_id: &str,
    connector_mandate_details: Option<storage::PaymentsMandateReference>,
) -> errors::RouterResult<bool> {
    if let Some(connector_mandate_details) = connector_mandate_details {
        let mcas = state
            .store
            .find_merchant_connector_account_by_merchant_id_and_disabled_list(
                merchant_id,
                true,
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
                id: merchant_id.to_string(),
            })?;

        let mut mca_ids = HashSet::new();
        let mcas = mcas
            .into_iter()
            .filter(|mca| mca.disabled == Some(true))
            .collect::<Vec<_>>();

        for mca in mcas {
            mca_ids.insert(mca.merchant_connector_id);
        }

        for mca_id in connector_mandate_details.keys() {
            if !mca_ids.contains(mca_id) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
pub async fn decrypt_generic_data<T>(
    data: Option<Encryption>,
    key: &[u8],
) -> errors::RouterResult<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    let decrypted_data = decrypt::<serde_json::Value, masking::WithType>(data, key)
        .await
        .change_context(errors::StorageError::DecryptionError)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to decrypt data")?;

    decrypted_data
        .map(|decrypted_data| decrypted_data.into_inner().expose())
        .map(|decrypted_value| decrypted_value.parse_value("generic_data"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse generic data value")
}

pub async fn get_card_details_with_locker_fallback(
    pm: &payment_method::PaymentMethod,
    key: &[u8],
    state: &routes::AppState,
) -> errors::RouterResult<Option<api::CardDetailFromLocker>> {
    let card_decrypted =
        decrypt::<serde_json::Value, masking::WithType>(pm.payment_method_data.clone(), key)
            .await
            .change_context(errors::StorageError::DecryptionError)
            .attach_printable("unable to decrypt card details")
            .ok()
            .flatten()
            .map(|x| x.into_inner().expose())
            .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
            .and_then(|pmd| match pmd {
                PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                _ => None,
            });

    Ok(if let Some(mut crd) = card_decrypted {
        if crd.saved_to_locker {
            crd.scheme.clone_from(&pm.scheme);
            Some(crd)
        } else {
            None
        }
    } else {
        Some(get_card_details_from_locker(state, pm).await?)
    })
}

pub async fn get_card_details_without_locker_fallback(
    pm: &payment_method::PaymentMethod,
    key: &[u8],
    state: &routes::AppState,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let card_decrypted =
        decrypt::<serde_json::Value, masking::WithType>(pm.payment_method_data.clone(), key)
            .await
            .change_context(errors::StorageError::DecryptionError)
            .attach_printable("unable to decrypt card details")
            .ok()
            .flatten()
            .map(|x| x.into_inner().expose())
            .and_then(|v| serde_json::from_value::<PaymentMethodsData>(v).ok())
            .and_then(|pmd| match pmd {
                PaymentMethodsData::Card(crd) => Some(api::CardDetailFromLocker::from(crd)),
                _ => None,
            });

    Ok(if let Some(mut crd) = card_decrypted {
        crd.scheme.clone_from(&pm.scheme);
        crd
    } else {
        get_card_details_from_locker(state, pm).await?
    })
}

pub async fn get_card_details_from_locker(
    state: &routes::AppState,
    pm: &storage::PaymentMethod,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let card = get_card_from_locker(
        state,
        &pm.customer_id,
        &pm.merchant_id,
        pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error getting card from card vault")?;

    payment_methods::get_card_detail(pm, card)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Get Card Details Failed")
}

pub async fn get_lookup_key_from_locker(
    state: &routes::AppState,
    payment_token: &str,
    pm: &storage::PaymentMethod,
    merchant_key_store: &domain::MerchantKeyStore,
) -> errors::RouterResult<api::CardDetailFromLocker> {
    let card_detail = get_card_details_from_locker(state, pm).await?;
    let card = card_detail.clone();

    let resp = TempLockerCardSupport::create_payment_method_data_in_temp_locker(
        state,
        payment_token,
        card,
        pm,
        merchant_key_store,
    )
    .await?;
    Ok(resp)
}

async fn get_masked_bank_details(
    pm: &payment_method::PaymentMethod,
    key: &[u8],
) -> errors::RouterResult<Option<MaskedBankDetails>> {
    let payment_method_data =
        decrypt::<serde_json::Value, masking::WithType>(pm.payment_method_data.clone(), key)
            .await
            .change_context(errors::StorageError::DecryptionError)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to decrypt bank details")?
            .map(|x| x.into_inner().expose())
            .map(
                |v| -> Result<PaymentMethodsData, error_stack::Report<errors::ApiErrorResponse>> {
                    v.parse_value::<PaymentMethodsData>("PaymentMethodsData")
                        .change_context(errors::StorageError::DeserializationFailed)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to deserialize Payment Method Auth config")
                },
            )
            .transpose()?;

    match payment_method_data {
        Some(pmd) => match pmd {
            PaymentMethodsData::Card(_) => Ok(None),
            PaymentMethodsData::BankDetails(bank_details) => Ok(Some(MaskedBankDetails {
                mask: bank_details.mask,
            })),
        },
        None => Err(report!(errors::ApiErrorResponse::InternalServerError))
            .attach_printable("Unable to fetch payment method data"),
    }
}

async fn get_bank_account_connector_details(
    pm: &payment_method::PaymentMethod,
    key: &[u8],
) -> errors::RouterResult<Option<BankAccountTokenData>> {
    let payment_method_data =
        decrypt::<serde_json::Value, masking::WithType>(pm.payment_method_data.clone(), key)
            .await
            .change_context(errors::StorageError::DecryptionError)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("unable to decrypt bank details")?
            .map(|x| x.into_inner().expose())
            .map(
                |v| -> Result<PaymentMethodsData, error_stack::Report<errors::ApiErrorResponse>> {
                    v.parse_value::<PaymentMethodsData>("PaymentMethodsData")
                        .change_context(errors::StorageError::DeserializationFailed)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to deserialize Payment Method Auth config")
                },
            )
            .transpose()?;

    match payment_method_data {
        Some(pmd) => match pmd {
            PaymentMethodsData::Card(_) => Err(errors::ApiErrorResponse::UnprocessableEntity {
                message: "Card is not a valid entity".to_string(),
            }
            .into()),
            PaymentMethodsData::BankDetails(bank_details) => {
                let connector_details = bank_details
                    .connector_details
                    .first()
                    .ok_or(errors::ApiErrorResponse::InternalServerError)?;

                let pm_type = pm
                    .payment_method_type
                    .get_required_value("payment_method_type")
                    .attach_printable("PaymentMethodType not found")?;

                let pm = pm
                    .payment_method
                    .get_required_value("payment_method")
                    .attach_printable("PaymentMethod not found")?;

                let token_data = BankAccountTokenData {
                    payment_method_type: pm_type,
                    payment_method: pm,
                    connector_details: connector_details.clone(),
                };

                Ok(Some(token_data))
            }
        },
        None => Ok(None),
    }
}
pub async fn set_default_payment_method(
    db: &dyn db::StorageInterface,
    merchant_id: String,
    key_store: domain::MerchantKeyStore,
    customer_id: &str,
    payment_method_id: String,
    storage_scheme: MerchantStorageScheme,
) -> errors::RouterResponse<CustomerDefaultPaymentMethodResponse> {
    //check for the customer
    let customer = db
        .find_customer_by_customer_id_merchant_id(
            customer_id,
            &merchant_id,
            &key_store,
            storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;
    // check for the presence of payment_method
    let payment_method = db
        .find_payment_method(&payment_method_id, storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    let pm = payment_method
        .payment_method
        .get_required_value("payment_method")?;

    utils::when(
        payment_method.customer_id != customer_id || payment_method.merchant_id != merchant_id,
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "The payment_method_id is not valid".to_string(),
            })
        },
    )?;

    utils::when(
        Some(payment_method_id.clone()) == customer.default_payment_method_id,
        || {
            Err(errors::ApiErrorResponse::PreconditionFailed {
                message: "Payment Method is already set as default".to_string(),
            })
        },
    )?;

    let customer_update = CustomerUpdate::UpdateDefaultPaymentMethod {
        default_payment_method_id: Some(Some(payment_method_id.to_owned())),
    };

    let customer_id = customer.customer_id.clone();

    // update the db with the default payment method id
    let updated_customer_details = db
        .update_customer_by_customer_id_merchant_id(
            customer_id.to_owned(),
            merchant_id.to_owned(),
            customer,
            customer_update,
            &key_store,
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update the default payment method id for the customer")?;

    let resp = CustomerDefaultPaymentMethodResponse {
        default_payment_method_id: updated_customer_details.default_payment_method_id,
        customer_id,
        payment_method_type: payment_method.payment_method_type,
        payment_method: pm,
    };

    Ok(services::ApplicationResponse::Json(resp))
}

pub async fn update_last_used_at(
    pm_id: &str,
    state: &routes::AppState,
    storage_scheme: MerchantStorageScheme,
) -> errors::RouterResult<()> {
    let update_last_used = storage::PaymentMethodUpdate::LastUsedUpdate {
        last_used_at: common_utils::date_time::now(),
    };
    let payment_method = state
        .store
        .find_payment_method(pm_id, storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    state
        .store
        .update_payment_method(payment_method, update_last_used, storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update the last_used_at in db")?;

    Ok(())
}
#[cfg(feature = "payouts")]
pub async fn get_bank_from_hs_locker(
    state: &routes::AppState,
    key_store: &domain::MerchantKeyStore,
    temp_token: &str,
    customer_id: &str,
    merchant_id: &str,
    token_ref: &str,
) -> errors::RouterResult<api::BankPayout> {
    let payment_method = get_payment_method_from_hs_locker(
        state,
        key_store,
        customer_id,
        merchant_id,
        token_ref,
        None,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Error getting payment method from locker")?;
    let pm_parsed: api::PayoutMethodData = payment_method
        .peek()
        .to_string()
        .parse_struct("PayoutMethodData")
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    match &pm_parsed {
        api::PayoutMethodData::Bank(bank) => {
            vault::Vault::store_payout_method_data_in_locker(
                state,
                Some(temp_token.to_string()),
                &pm_parsed,
                Some(customer_id.to_owned()),
                key_store,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error storing payout method data in temporary locker")?;
            Ok(bank.to_owned())
        }
        api::PayoutMethodData::Card(_) => Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Expected bank details, found card details instead".to_string(),
        }
        .into()),
        api::PayoutMethodData::Wallet(_) => Err(errors::ApiErrorResponse::InvalidRequestData {
            message: "Expected bank details, found wallet details instead".to_string(),
        }
        .into()),
    }
}

pub struct TempLockerCardSupport;

impl TempLockerCardSupport {
    #[instrument(skip_all)]
    async fn create_payment_method_data_in_temp_locker(
        state: &routes::AppState,
        payment_token: &str,
        card: api::CardDetailFromLocker,
        pm: &storage::PaymentMethod,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> errors::RouterResult<api::CardDetailFromLocker> {
        let card_number = card.card_number.clone().get_required_value("card_number")?;
        let card_exp_month = card
            .expiry_month
            .clone()
            .expose_option()
            .get_required_value("expiry_month")?;
        let card_exp_year = card
            .expiry_year
            .clone()
            .expose_option()
            .get_required_value("expiry_year")?;
        let card_holder_name = card
            .card_holder_name
            .clone()
            .expose_option()
            .unwrap_or_default();
        let value1 = payment_methods::mk_card_value1(
            card_number,
            card_exp_year,
            card_exp_month,
            Some(card_holder_name),
            None,
            None,
            None,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value1 for locker")?;
        let value2 = payment_methods::mk_card_value2(
            None,
            None,
            None,
            Some(pm.customer_id.to_string()),
            Some(pm.payment_method_id.to_string()),
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error getting Value2 for locker")?;

        let value1 = vault::VaultPaymentMethod::Card(value1);
        let value2 = vault::VaultPaymentMethod::Card(value2);

        let value1 = value1
            .encode_to_string_of_json()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value1 construction failed when saving card to locker")?;

        let value2 = value2
            .encode_to_string_of_json()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Wrapped value2 construction failed when saving card to locker")?;

        let lookup_key = vault::create_tokenize(
            state,
            value1,
            Some(value2),
            payment_token.to_string(),
            merchant_key_store.key.get_inner(),
        )
        .await?;
        vault::add_delete_tokenized_data_task(
            &*state.store,
            &lookup_key,
            enums::PaymentMethod::Card,
        )
        .await?;
        metrics::TOKENIZED_DATA_COUNT.add(&metrics::CONTEXT, 1, &[]);
        metrics::TASKS_ADDED_COUNT.add(
            &metrics::CONTEXT,
            1,
            &[request::add_attributes("flow", "DeleteTokenizeData")],
        );
        Ok(card)
    }
}

#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: routes::AppState,
    pm: api::PaymentMethodId,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    let db = state.store.as_ref();
    let pm = db
        .find_payment_method(&pm.payment_method_id, merchant_account.storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let key = key_store.key.peek();
    let card = if pm.payment_method == Some(enums::PaymentMethod::Card) {
        let card_detail = if state.conf.locker.locker_enabled {
            let card = get_card_from_locker(
                &state,
                &pm.customer_id,
                &pm.merchant_id,
                pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error getting card from card vault")?;
            payment_methods::get_card_detail(&pm, card)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed while getting card details from locker")?
        } else {
            get_card_details_without_locker_fallback(&pm, key, &state).await?
        };
        Some(card_detail)
    } else {
        None
    };
    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodResponse {
            merchant_id: pm.merchant_id,
            customer_id: Some(pm.customer_id),
            payment_method_id: pm.payment_method_id,
            payment_method: pm.payment_method,
            payment_method_type: pm.payment_method_type,
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            card,
            metadata: pm.metadata,
            created: Some(pm.created_at),
            recurring_enabled: false,
            installment_payment_enabled: false,
            payment_experience: Some(vec![api_models::enums::PaymentExperience::RedirectToUrl]),
            last_used_at: Some(pm.last_used_at),
            client_secret: pm.client_secret,
        },
    ))
}

#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    pm_id: api::PaymentMethodId,
    key_store: domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodDeleteResponse> {
    let db = state.store.as_ref();
    let key = db
        .find_payment_method(
            pm_id.payment_method_id.as_str(),
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let payment_methods_count = db
        .get_payment_method_count_by_customer_id_merchant_id_status(
            &key.customer_id,
            &merchant_account.merchant_id,
            api_enums::PaymentMethodStatus::Active,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get a count of payment methods for a customer")?;

    let customer = db
        .find_customer_by_customer_id_merchant_id(
            &key.customer_id,
            &merchant_account.merchant_id,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Customer not found for the payment method")?;

    utils::when(
        customer.default_payment_method_id.as_ref() == Some(&pm_id.payment_method_id)
            && payment_methods_count > 1,
        || Err(errors::ApiErrorResponse::PaymentMethodDeleteFailed),
    )?;

    if key.payment_method == Some(enums::PaymentMethod::Card) {
        let response = delete_card_from_locker(
            &state,
            &key.customer_id,
            &key.merchant_id,
            key.locker_id.as_ref().unwrap_or(&key.payment_method_id),
        )
        .await?;

        if response.status == "Ok" {
            logger::info!("Card From locker deleted Successfully!");
        } else {
            logger::error!("Error: Deleting Card From Locker!\n{:#?}", response);
            Err(errors::ApiErrorResponse::InternalServerError)?
        }
    }

    db.delete_payment_method_by_merchant_id_payment_method_id(
        &merchant_account.merchant_id,
        pm_id.payment_method_id.as_str(),
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    if customer.default_payment_method_id.as_ref() == Some(&pm_id.payment_method_id) {
        let customer_update = CustomerUpdate::UpdateDefaultPaymentMethod {
            default_payment_method_id: Some(None),
        };

        db.update_customer_by_customer_id_merchant_id(
            key.customer_id,
            key.merchant_id,
            customer,
            customer_update,
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update the default payment method id for the customer")?;
    };

    Ok(services::ApplicationResponse::Json(
        api::PaymentMethodDeleteResponse {
            payment_method_id: key.payment_method_id,
            deleted: true,
        },
    ))
}

pub async fn create_encrypted_data<T>(
    key_store: &domain::MerchantKeyStore,
    data: Option<T>,
) -> Option<Encryption>
where
    T: Debug + serde::Serialize,
{
    let key = key_store.key.get_inner().peek();

    let encrypted_data: Option<Encryption> = data
        .as_ref()
        .map(Encode::encode_to_value)
        .transpose()
        .change_context(errors::StorageError::SerializationFailed)
        .attach_printable("Unable to convert data to a value")
        .unwrap_or_else(|err| {
            logger::error!(err=?err);
            None
        })
        .map(Secret::<_, masking::WithType>::new)
        .async_lift(|inner| encrypt_optional(inner, key))
        .await
        .change_context(errors::StorageError::EncryptionError)
        .attach_printable("Unable to encrypt data")
        .unwrap_or_else(|err| {
            logger::error!(err=?err);
            None
        })
        .map(|details| details.into());

    encrypted_data
}

pub async fn list_countries_currencies_for_connector_payment_method(
    state: routes::AppState,
    req: ListCountriesCurrenciesRequest,
) -> errors::RouterResponse<ListCountriesCurrenciesResponse> {
    Ok(services::ApplicationResponse::Json(
        list_countries_currencies_for_connector_payment_method_util(
            state.conf.pm_filters.clone(),
            req.connector,
            req.payment_method_type,
        )
        .await,
    ))
}

// This feature will be more efficient as a WASM function rather than as an API.
// So extracting this logic to a separate function so that it can be used in WASM as well.
pub async fn list_countries_currencies_for_connector_payment_method_util(
    connector_filters: settings::ConnectorFilters,
    connector: api_enums::Connector,
    payment_method_type: api_enums::PaymentMethodType,
) -> ListCountriesCurrenciesResponse {
    let payment_method_type =
        settings::PaymentMethodFilterKey::PaymentMethodType(payment_method_type);

    let (currencies, country_codes) = connector_filters
        .0
        .get(&connector.to_string())
        .and_then(|filter| filter.0.get(&payment_method_type))
        .map(|filter| (filter.currency.clone(), filter.country.clone()))
        .unwrap_or_else(|| {
            connector_filters
                .0
                .get("default")
                .and_then(|filter| filter.0.get(&payment_method_type))
                .map_or((None, None), |filter| {
                    (filter.currency.clone(), filter.country.clone())
                })
        });

    let currencies =
        currencies.unwrap_or_else(|| api_enums::Currency::iter().collect::<HashSet<_>>());
    let country_codes =
        country_codes.unwrap_or_else(|| api_enums::CountryAlpha2::iter().collect::<HashSet<_>>());

    ListCountriesCurrenciesResponse {
        currencies,
        countries: country_codes
            .into_iter()
            .map(|country_code| CountryCodeWithName {
                code: country_code,
                name: common_enums::Country::from_alpha2(country_code),
            })
            .collect(),
    }
}
