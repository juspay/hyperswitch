pub mod cards;
pub mod surcharge_decision_configs;
pub mod transformers;
pub mod vault;
pub use api_models::enums::Connector;
#[cfg(feature = "payouts")]
pub use api_models::{enums::PayoutConnectors, payouts as payout_types};
use api_models::{payment_methods, payments::CardToken};
use data_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use diesel_models::{enums, GenericLinkNew, GenericLinkS, PaymentMethodCollectLinkData};
use error_stack::{report, ResultExt};

use super::errors::{RouterResponse, StorageErrorExt};
use crate::{
    core::{
        errors::{self, RouterResult},
        payments::helpers,
        pm_auth as core_pm_auth,
    },
    routes::{app::StorageInterface, AppState},
    services::{self, GenericLinks},
    types::{
        api::{self, payments},
        domain, storage,
    },
};
mod validator;
pub struct Oss;

#[async_trait::async_trait]
pub trait PaymentMethodRetrieve {
    async fn retrieve_payment_method(
        pm_data: &Option<payments::PaymentMethodData>,
        state: &AppState,
        payment_intent: &PaymentIntent,
        payment_attempt: &PaymentAttempt,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(Option<payments::PaymentMethodData>, Option<String>)>;

    async fn retrieve_payment_method_with_token(
        state: &AppState,
        key_store: &domain::MerchantKeyStore,
        token: &storage::PaymentTokenData,
        payment_intent: &PaymentIntent,
        card_token_data: Option<&CardToken>,
        customer: &Option<domain::Customer>,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> RouterResult<storage::PaymentMethodDataWithId>;
}

#[async_trait::async_trait]
impl PaymentMethodRetrieve for Oss {
    async fn retrieve_payment_method(
        pm_data: &Option<payments::PaymentMethodData>,
        state: &AppState,
        payment_intent: &PaymentIntent,
        payment_attempt: &PaymentAttempt,
        merchant_key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<(Option<payments::PaymentMethodData>, Option<String>)> {
        match pm_data {
            pm_opt @ Some(pm @ api::PaymentMethodData::Card(_)) => {
                let payment_token = helpers::store_payment_method_data_in_vault(
                    state,
                    payment_attempt,
                    payment_intent,
                    enums::PaymentMethod::Card,
                    pm,
                    merchant_key_store,
                )
                .await?;

                Ok((pm_opt.to_owned(), payment_token))
            }
            pm @ Some(api::PaymentMethodData::PayLater(_)) => Ok((pm.to_owned(), None)),
            pm @ Some(api::PaymentMethodData::Crypto(_)) => Ok((pm.to_owned(), None)),
            pm @ Some(api::PaymentMethodData::BankDebit(_)) => Ok((pm.to_owned(), None)),
            pm @ Some(api::PaymentMethodData::Upi(_)) => Ok((pm.to_owned(), None)),
            pm @ Some(api::PaymentMethodData::Voucher(_)) => Ok((pm.to_owned(), None)),
            pm @ Some(api::PaymentMethodData::Reward) => Ok((pm.to_owned(), None)),
            pm @ Some(api::PaymentMethodData::CardRedirect(_)) => Ok((pm.to_owned(), None)),
            pm @ Some(api::PaymentMethodData::GiftCard(_)) => Ok((pm.to_owned(), None)),
            pm_opt @ Some(pm @ api::PaymentMethodData::BankTransfer(_)) => {
                let payment_token = helpers::store_payment_method_data_in_vault(
                    state,
                    payment_attempt,
                    payment_intent,
                    enums::PaymentMethod::BankTransfer,
                    pm,
                    merchant_key_store,
                )
                .await?;

                Ok((pm_opt.to_owned(), payment_token))
            }
            pm_opt @ Some(pm @ api::PaymentMethodData::Wallet(_)) => {
                let payment_token = helpers::store_payment_method_data_in_vault(
                    state,
                    payment_attempt,
                    payment_intent,
                    enums::PaymentMethod::Wallet,
                    pm,
                    merchant_key_store,
                )
                .await?;

                Ok((pm_opt.to_owned(), payment_token))
            }
            pm_opt @ Some(pm @ api::PaymentMethodData::BankRedirect(_)) => {
                let payment_token = helpers::store_payment_method_data_in_vault(
                    state,
                    payment_attempt,
                    payment_intent,
                    enums::PaymentMethod::BankRedirect,
                    pm,
                    merchant_key_store,
                )
                .await?;

                Ok((pm_opt.to_owned(), payment_token))
            }
            _ => Ok((None, None)),
        }
    }

    async fn retrieve_payment_method_with_token(
        state: &AppState,
        merchant_key_store: &domain::MerchantKeyStore,
        token_data: &storage::PaymentTokenData,
        payment_intent: &PaymentIntent,
        card_token_data: Option<&CardToken>,
        customer: &Option<domain::Customer>,
        storage_scheme: common_enums::enums::MerchantStorageScheme,
    ) -> RouterResult<storage::PaymentMethodDataWithId> {
        let token = match token_data {
            storage::PaymentTokenData::TemporaryGeneric(generic_token) => {
                helpers::retrieve_payment_method_with_temporary_token(
                    state,
                    &generic_token.token,
                    payment_intent,
                    merchant_key_store,
                    card_token_data,
                )
                .await?
                .map(
                    |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                        payment_method_data: Some(payment_method_data),
                        payment_method: Some(payment_method),
                        payment_method_id: None,
                    },
                )
                .unwrap_or_default()
            }

            storage::PaymentTokenData::Temporary(generic_token) => {
                helpers::retrieve_payment_method_with_temporary_token(
                    state,
                    &generic_token.token,
                    payment_intent,
                    merchant_key_store,
                    card_token_data,
                )
                .await?
                .map(
                    |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                        payment_method_data: Some(payment_method_data),
                        payment_method: Some(payment_method),
                        payment_method_id: None,
                    },
                )
                .unwrap_or_default()
            }

            storage::PaymentTokenData::Permanent(card_token) => {
                helpers::retrieve_card_with_permanent_token(
                    state,
                    card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                    card_token
                        .payment_method_id
                        .as_ref()
                        .unwrap_or(&card_token.token),
                    payment_intent,
                    card_token_data,
                    merchant_key_store,
                    storage_scheme,
                )
                .await
                .map(|card| Some((card, enums::PaymentMethod::Card)))?
                .map(
                    |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                        payment_method_data: Some(payment_method_data),
                        payment_method: Some(payment_method),
                        payment_method_id: Some(
                            card_token
                                .payment_method_id
                                .as_ref()
                                .unwrap_or(&card_token.token)
                                .to_string(),
                        ),
                    },
                )
                .unwrap_or_default()
            }

            storage::PaymentTokenData::PermanentCard(card_token) => {
                helpers::retrieve_card_with_permanent_token(
                    state,
                    card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                    card_token
                        .payment_method_id
                        .as_ref()
                        .unwrap_or(&card_token.token),
                    payment_intent,
                    card_token_data,
                    merchant_key_store,
                    storage_scheme,
                )
                .await
                .map(|card| Some((card, enums::PaymentMethod::Card)))?
                .map(
                    |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                        payment_method_data: Some(payment_method_data),
                        payment_method: Some(payment_method),
                        payment_method_id: Some(
                            card_token
                                .payment_method_id
                                .as_ref()
                                .unwrap_or(&card_token.token)
                                .to_string(),
                        ),
                    },
                )
                .unwrap_or_default()
            }

            storage::PaymentTokenData::AuthBankDebit(auth_token) => {
                core_pm_auth::retrieve_payment_method_from_auth_service(
                    state,
                    merchant_key_store,
                    auth_token,
                    payment_intent,
                    customer,
                )
                .await?
                .map(
                    |(payment_method_data, payment_method)| storage::PaymentMethodDataWithId {
                        payment_method_data: Some(payment_method_data),
                        payment_method: Some(payment_method),
                        payment_method_id: None,
                    },
                )
                .unwrap_or_default()
            }

            storage::PaymentTokenData::WalletToken(_) => storage::PaymentMethodDataWithId {
                payment_method: None,
                payment_method_data: None,
                payment_method_id: None,
            },
        };
        Ok(token)
    }
}

pub async fn initiate_pm_collect_link(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payment_methods::PaymentMethodCollectLinkRequest,
) -> RouterResponse<payment_methods::PaymentMethodCollectLinkResponse> {
    // Validate request and initiate flow
    let pm_collect_link_data =
        validator::validate_request_and_initiate_payment_method_collect_link(
            &state,
            &merchant_account,
            &key_store,
            &req,
        )
        .await?;

    // Create DB entries
    let generic_link_db =
        create_pm_collect_db_entry(&state, &merchant_account, &pm_collect_link_data).await?;

    // Return response
    let response = payment_methods::PaymentMethodCollectLinkResponse {
        pm_collect_link_id: generic_link_db.link_id,
        customer_id: generic_link_db.primary_reference,
        link: generic_link_db.url,
    };
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn create_pm_collect_db_entry(
    state: &AppState,
    merchant_account: &domain::MerchantAccount,
    pm_collect_link_data: &PaymentMethodCollectLinkData,
) -> RouterResult<GenericLinkS> {
    let db: &dyn StorageInterface = &*state.store;

    let link_data = serde_json::to_value(pm_collect_link_data)
        .map_err(|_| report!(errors::ApiErrorResponse::InternalServerError))
        .attach_printable("Failed to convert PaymentMethodCollectLinkData to Value")?;

    let generic_link_new = GenericLinkNew {
        link_id: pm_collect_link_data.pm_collect_link_id.to_string(),
        primary_reference: pm_collect_link_data.customer_id.to_string(),
        merchant_id: merchant_account.merchant_id.to_string(),
        link_type: common_enums::GenericLinkType::PaymentMethodCollect,
        link_data,
        url: pm_collect_link_data.link.to_string(),
        ..Default::default()
    };

    db.insert_generic_link(generic_link_new)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "payment method collect link already exists".to_string(),
        })
}

pub async fn render_pm_collect_link(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payment_methods::PaymentMethodCollectLinkRenderRequest,
) -> RouterResponse<services::GenericLinkFormData> {
    let db: &dyn StorageInterface = &*state.store;

    // Fetch pm collect link
    let pm_collect_link = db
        .find_generic_link_by_link_id(&req.pm_collect_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment method collect link not found".to_string(),
        })?;

    // Check status and return form data accordingly
    let has_expired = common_utils::date_time::now() > pm_collect_link.expiry;
    let status = pm_collect_link.link_status;
    match status {
        enums::GenericLinkStatus::PaymentMethodCollect(
            enums::PaymentMethodCollectStatus::Initiated,
        ) => {
            // if expired, send back expired status page
            if has_expired {
                todo!()

            // else, send back form link
            } else {
                // Fetch customer
                let customer = db
                    .find_customer_by_customer_id_merchant_id(
                        &pm_collect_link.primary_reference,
                        &req.merchant_id,
                        &key_store,
                        merchant_account.storage_scheme,
                    )
                    .await
                    .change_context(errors::ApiErrorResponse::InvalidRequestData {
                        message: format!(
                            "Customer [{}] not found for link_id - {}",
                            pm_collect_link.primary_reference, pm_collect_link.link_id
                        ),
                    })
                    .attach_printable(format!(
                        "customer [{}] not found",
                        pm_collect_link.primary_reference
                    ))?;

                let link_data = pm_collect_link
                    .link_data
                    .get_payment_method_collect_data()
                    .map_err(|e| report!(errors::ApiErrorResponse::InternalServerError))?;

                let theme = payment_methods::PaymentMethodCollectLinkTheme {
                    primary_color: "#1a1a1a".to_string(),
                };

                let js_data = payment_methods::PaymentMethodCollectLinkDetails {
                    pub_key: merchant_account.publishable_key.ok_or(
                        errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "pub_key",
                        },
                    )?,
                    client_secret: link_data.client_secret.clone(),
                    pm_collect_link_id: pm_collect_link.link_id,
                    customer_id: customer.customer_id,
                    session_expiry: pm_collect_link.expiry,
                    return_url: "".to_string(),
                    theme,
                };

                let css_data = "";

                let generic_form_data = services::GenericLinkFormData {
                    js_data: serialize_dynamic_content(&js_data)?,
                    css_data: css_data.to_string(),
                    sdk_url: state
                        .conf
                        .generic_link
                        .payment_method_collect
                        .sdk_url
                        .clone(),
                    html_meta_tags: "".to_string(),
                };
                Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                    GenericLinks::PaymentMethodCollect(generic_form_data),
                )))
            }
        }
        enums::GenericLinkStatus::PaymentMethodCollect(
            enums::PaymentMethodCollectStatus::Invalidated,
        ) => {
            // Send back expired status page
            todo!()
        }
        enums::GenericLinkStatus::PaymentMethodCollect(
            enums::PaymentMethodCollectStatus::Submitted,
        ) => {
            // Send back status page
            todo!()
        }
    }
}

fn serialize_dynamic_content(
    js_data: &payment_methods::PaymentMethodCollectLinkDetails,
) -> RouterResult<String> {
    let js_data_str = serde_json::to_string(js_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize PaymentMethodCollectLinkDetails")?;
    Ok(format!("window.__PM_COLLECT_DETAILS = {js_data_str};"))
}
