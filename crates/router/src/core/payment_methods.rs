pub mod cards;
pub mod helpers;
pub mod migration;
pub mod surcharge_decision_configs;
pub mod transformers;
pub mod utils;
mod validator;
pub mod vault;

use std::borrow::Cow;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use std::collections::HashSet;
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use std::str::FromStr;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub use api_models::enums as api_enums;
pub use api_models::enums::Connector;
use api_models::payment_methods;
#[cfg(feature = "payouts")]
pub use api_models::{enums::PayoutConnectors, payouts as payout_types};
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use common_utils::ext_traits::Encode;
use common_utils::{consts::DEFAULT_LOCALE, id_type::CustomerId};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::{ext_traits::Encode, id_type, request::RequestContent};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::{ext_traits::OptionExt, generate_id};
use diesel_models::{
    enums, GenericLinkNew, PaymentMethodCollectLink, PaymentMethodCollectLinkData,
};
use error_stack::{report, ResultExt};
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use hyperswitch_domain_models::api::{GenericLinks, GenericLinksData};
use hyperswitch_domain_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
use masking::PeekInterface;
use router_env::{instrument, tracing};
use time::Duration;

use super::errors::{RouterResponse, StorageErrorExt};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::{
    configs::settings, core::payment_methods::transformers as pm_transforms, headers,
    services::encryption, types::api,
};
use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments::helpers as payment_helpers,
        pm_auth as core_pm_auth,
    },
    routes::{app::StorageInterface, SessionState},
    services,
    types::{
        domain,
        storage::{self, enums as storage_enums},
    },
};

const PAYMENT_METHOD_STATUS_UPDATE_TASK: &str = "PAYMENT_METHOD_STATUS_UPDATE";
const PAYMENT_METHOD_STATUS_TAG: &str = "PAYMENT_METHOD_STATUS";

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
const ADD_VAULT_REQUEST_URL: &str = "/vault/add";
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
const VAULT_FINGERPRINT_REQUEST_URL: &str = "/fingerprint";
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
const VAULT_HEADER_CONTENT_TYPE: &str = "application/json";

#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    pm_data: &Option<domain::PaymentMethodData>,
    state: &SessionState,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    merchant_key_store: &domain::MerchantKeyStore,
    business_profile: Option<&domain::BusinessProfile>,
) -> RouterResult<(Option<domain::PaymentMethodData>, Option<String>)> {
    match pm_data {
        pm_opt @ Some(pm @ domain::PaymentMethodData::Card(_)) => {
            let payment_token = payment_helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::Card,
                pm,
                merchant_key_store,
                business_profile,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        pm @ Some(domain::PaymentMethodData::PayLater(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::Crypto(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::BankDebit(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::Upi(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::Voucher(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::Reward) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::RealTimePayment(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::CardRedirect(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::GiftCard(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::OpenBanking(_)) => Ok((pm.to_owned(), None)),
        pm_opt @ Some(pm @ domain::PaymentMethodData::BankTransfer(_)) => {
            let payment_token = payment_helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::BankTransfer,
                pm,
                merchant_key_store,
                business_profile,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        pm_opt @ Some(pm @ domain::PaymentMethodData::Wallet(_)) => {
            let payment_token = payment_helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::Wallet,
                pm,
                merchant_key_store,
                business_profile,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        pm_opt @ Some(pm @ domain::PaymentMethodData::BankRedirect(_)) => {
            let payment_token = payment_helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::BankRedirect,
                pm,
                merchant_key_store,
                business_profile,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        _ => Ok((None, None)),
    }
}

pub async fn initiate_pm_collect_link(
    state: SessionState,
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
    let pm_collect_link = create_pm_collect_db_entry(
        &state,
        &merchant_account,
        &pm_collect_link_data,
        req.return_url.clone(),
    )
    .await?;
    let customer_id = CustomerId::try_from(Cow::from(pm_collect_link.primary_reference))
        .change_context(errors::ApiErrorResponse::InvalidDataValue {
            field_name: "customer_id",
        })?;

    // Return response
    let url = pm_collect_link.url.peek();
    let response = payment_methods::PaymentMethodCollectLinkResponse {
        pm_collect_link_id: pm_collect_link.link_id,
        customer_id,
        expiry: pm_collect_link.expiry,
        link: url::Url::parse(url)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable_lazy(|| {
                format!("Failed to parse the payment method collect link - {}", url)
            })?
            .into(),
        return_url: pm_collect_link.return_url,
        ui_config: pm_collect_link.link_data.ui_config,
        enabled_payment_methods: pm_collect_link.link_data.enabled_payment_methods,
    };
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn create_pm_collect_db_entry(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    pm_collect_link_data: &PaymentMethodCollectLinkData,
    return_url: Option<String>,
) -> RouterResult<PaymentMethodCollectLink> {
    let db: &dyn StorageInterface = &*state.store;

    let link_data = serde_json::to_value(pm_collect_link_data)
        .map_err(|_| report!(errors::ApiErrorResponse::InternalServerError))
        .attach_printable("Failed to convert PaymentMethodCollectLinkData to Value")?;

    let pm_collect_link = GenericLinkNew {
        link_id: pm_collect_link_data.pm_collect_link_id.to_string(),
        primary_reference: pm_collect_link_data
            .customer_id
            .get_string_repr()
            .to_string(),
        merchant_id: merchant_account.get_id().to_owned(),
        link_type: common_enums::GenericLinkType::PaymentMethodCollect,
        link_data,
        url: pm_collect_link_data.link.clone(),
        return_url,
        expiry: common_utils::date_time::now()
            + Duration::seconds(pm_collect_link_data.session_expiry.into()),
        ..Default::default()
    };

    db.insert_pm_collect_link(pm_collect_link)
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "payment method collect link already exists".to_string(),
        })
}

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub async fn render_pm_collect_link(
    _state: SessionState,
    _merchant_account: domain::MerchantAccount,
    _key_store: domain::MerchantKeyStore,
    _req: payment_methods::PaymentMethodCollectLinkRenderRequest,
) -> RouterResponse<services::GenericLinkFormData> {
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub async fn render_pm_collect_link(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: payment_methods::PaymentMethodCollectLinkRenderRequest,
) -> RouterResponse<services::GenericLinkFormData> {
    let db: &dyn StorageInterface = &*state.store;

    // Fetch pm collect link
    let pm_collect_link = db
        .find_pm_collect_link_by_link_id(&req.pm_collect_link_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment method collect link not found".to_string(),
        })?;

    // Check status and return form data accordingly
    let has_expired = common_utils::date_time::now() > pm_collect_link.expiry;
    let status = pm_collect_link.link_status;
    let link_data = pm_collect_link.link_data;
    let default_config = &state.conf.generic_link.payment_method_collect;
    let default_ui_config = default_config.ui_config.clone();
    let ui_config_data = common_utils::link_utils::GenericLinkUiConfigFormData {
        merchant_name: link_data
            .ui_config
            .merchant_name
            .unwrap_or(default_ui_config.merchant_name),
        logo: link_data.ui_config.logo.unwrap_or(default_ui_config.logo),
        theme: link_data
            .ui_config
            .theme
            .clone()
            .unwrap_or(default_ui_config.theme.clone()),
    };
    match status {
        common_utils::link_utils::PaymentMethodCollectStatus::Initiated => {
            // if expired, send back expired status page
            if has_expired {
                let expired_link_data = services::GenericExpiredLinkData {
                    title: "Payment collect link has expired".to_string(),
                    message: "This payment collect link has expired.".to_string(),
                    theme: link_data.ui_config.theme.unwrap_or(default_ui_config.theme),
                };
                Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                    GenericLinks {
                        allowed_domains: HashSet::from([]),
                        data: GenericLinksData::ExpiredLink(expired_link_data),
                        locale: DEFAULT_LOCALE.to_string(),
                    },
                )))

            // else, send back form link
            } else {
                let customer_id =
                    CustomerId::try_from(Cow::from(pm_collect_link.primary_reference.clone()))
                        .change_context(errors::ApiErrorResponse::InvalidDataValue {
                            field_name: "customer_id",
                        })?;
                // Fetch customer

                let customer = db
                    .find_customer_by_customer_id_merchant_id(
                        &(&state).into(),
                        &customer_id,
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

                let js_data = payment_methods::PaymentMethodCollectLinkDetails {
                    publishable_key: masking::Secret::new(merchant_account.publishable_key),
                    client_secret: link_data.client_secret.clone(),
                    pm_collect_link_id: pm_collect_link.link_id,
                    customer_id: customer.customer_id,
                    session_expiry: pm_collect_link.expiry,
                    return_url: pm_collect_link.return_url,
                    ui_config: ui_config_data,
                    enabled_payment_methods: link_data.enabled_payment_methods,
                };

                let serialized_css_content = String::new();

                let serialized_js_content = format!(
                    "window.__PM_COLLECT_DETAILS = {}",
                    js_data
                        .encode_to_string_of_json()
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to serialize PaymentMethodCollectLinkDetails")?
                );

                let generic_form_data = services::GenericLinkFormData {
                    js_data: serialized_js_content,
                    css_data: serialized_css_content,
                    sdk_url: default_config.sdk_url.to_string(),
                    html_meta_tags: String::new(),
                };
                Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                    GenericLinks {
                        allowed_domains: HashSet::from([]),
                        data: GenericLinksData::PaymentMethodCollect(generic_form_data),
                        locale: DEFAULT_LOCALE.to_string(),
                    },
                )))
            }
        }

        // Send back status page
        status => {
            let js_data = payment_methods::PaymentMethodCollectLinkStatusDetails {
                pm_collect_link_id: pm_collect_link.link_id,
                customer_id: link_data.customer_id,
                session_expiry: pm_collect_link.expiry,
                return_url: pm_collect_link
                    .return_url
                    .as_ref()
                    .map(|url| url::Url::parse(url))
                    .transpose()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to parse return URL for payment method collect's status link",
                    )?,
                ui_config: ui_config_data,
                status,
            };

            let serialized_css_content = String::new();

            let serialized_js_content = format!(
                "window.__PM_COLLECT_DETAILS = {}",
                js_data
                    .encode_to_string_of_json()
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable(
                        "Failed to serialize PaymentMethodCollectLinkStatusDetails"
                    )?
            );

            let generic_status_data = services::GenericLinkStatusData {
                js_data: serialized_js_content,
                css_data: serialized_css_content,
            };
            Ok(services::ApplicationResponse::GenericLinkForm(Box::new(
                GenericLinks {
                    allowed_domains: HashSet::from([]),
                    data: GenericLinksData::PaymentMethodCollectStatus(generic_status_data),
                    locale: DEFAULT_LOCALE.to_string(),
                },
            )))
        }
    }
}

fn generate_task_id_for_payment_method_status_update_workflow(
    key_id: &str,
    runner: &storage::ProcessTrackerRunner,
    task: &str,
) -> String {
    format!("{runner}_{task}_{key_id}")
}

pub async fn add_payment_method_status_update_task(
    db: &dyn StorageInterface,
    payment_method: &domain::PaymentMethod,
    prev_status: enums::PaymentMethodStatus,
    curr_status: enums::PaymentMethodStatus,
    merchant_id: &common_utils::id_type::MerchantId,
) -> Result<(), errors::ProcessTrackerError> {
    let created_at = payment_method.created_at;
    let schedule_time =
        created_at.saturating_add(Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

    let tracking_data = storage::PaymentMethodStatusTrackingData {
        payment_method_id: payment_method.get_id().clone(),
        prev_status,
        curr_status,
        merchant_id: merchant_id.to_owned(),
    };

    let runner = storage::ProcessTrackerRunner::PaymentMethodStatusUpdateWorkflow;
    let task = PAYMENT_METHOD_STATUS_UPDATE_TASK;
    let tag = [PAYMENT_METHOD_STATUS_TAG];

    let process_tracker_id = generate_task_id_for_payment_method_status_update_workflow(
        payment_method.get_id().as_str(),
        &runner,
        task,
    );
    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        task,
        runner,
        tag,
        tracking_data,
        schedule_time,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct PAYMENT_METHOD_STATUS_UPDATE process tracker task")?;

    db
        .insert_process(process_tracker_entry)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable_lazy(|| {
            format!(
                "Failed while inserting PAYMENT_METHOD_STATUS_UPDATE reminder to process_tracker for payment_method_id: {}",
                payment_method.get_id().clone()
            )
        })?;

    Ok(())
}

#[instrument(skip_all)]
pub async fn retrieve_payment_method_with_token(
    state: &SessionState,
    merchant_key_store: &domain::MerchantKeyStore,
    token_data: &storage::PaymentTokenData,
    payment_intent: &PaymentIntent,
    card_token_data: Option<&domain::CardToken>,
    customer: &Option<domain::Customer>,
    storage_scheme: common_enums::enums::MerchantStorageScheme,
) -> RouterResult<storage::PaymentMethodDataWithId> {
    let token = match token_data {
        storage::PaymentTokenData::TemporaryGeneric(generic_token) => {
            payment_helpers::retrieve_payment_method_with_temporary_token(
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
            payment_helpers::retrieve_payment_method_with_temporary_token(
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
            payment_helpers::retrieve_card_with_permanent_token(
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
            payment_helpers::retrieve_card_with_permanent_token(
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

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub(crate) async fn get_payment_method_create_request(
    payment_method_data: Option<&domain::PaymentMethodData>,
    payment_method: Option<storage_enums::PaymentMethod>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    customer_id: &Option<CustomerId>,
    billing_name: Option<masking::Secret<String>>,
) -> RouterResult<payment_methods::PaymentMethodCreate> {
    match payment_method_data {
        Some(pm_data) => match payment_method {
            Some(payment_method) => match pm_data {
                domain::PaymentMethodData::Card(card) => {
                    let card_detail = payment_methods::CardDetail {
                        card_number: card.card_number.clone(),
                        card_exp_month: card.card_exp_month.clone(),
                        card_exp_year: card.card_exp_year.clone(),
                        card_holder_name: billing_name,
                        nick_name: card.nick_name.clone(),
                        card_issuing_country: card
                            .card_issuing_country
                            .as_ref()
                            .map(|c| api_enums::CountryAlpha2::from_str(c))
                            .transpose()
                            .ok()
                            .flatten(),
                        card_network: card.card_network.clone(),
                        card_issuer: card.card_issuer.clone(),
                        card_type: card
                            .card_type
                            .as_ref()
                            .map(|c| payment_methods::CardType::from_str(c))
                            .transpose()
                            .ok()
                            .flatten(),
                    };
                    let payment_method_request = payment_methods::PaymentMethodCreate {
                        payment_method,
                        payment_method_type: payment_method_type
                            .get_required_value("Payment_method_type")
                            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                                field_name: "payment_method_data",
                            })?,
                        metadata: None,
                        customer_id: customer_id
                            .clone()
                            .get_required_value("customer_id")
                            .change_context(errors::ApiErrorResponse::MissingRequiredField {
                                field_name: "customer_id",
                            })?,
                        payment_method_data: payment_methods::PaymentMethodCreateData::Card(
                            card_detail,
                        ),
                        billing: None,
                    };
                    Ok(payment_method_request)
                }
                _ => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "payment_method_data"
                })
                .attach_printable("Payment method data is incorrect")),
            },
            None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_type"
            })
            .attach_printable("PaymentMethodType Required")),
        },
        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_method_data"
        })
        .attach_printable("PaymentMethodData required Or Card is already saved")),
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
pub(crate) async fn get_payment_method_create_request(
    payment_method_data: Option<&domain::PaymentMethodData>,
    payment_method: Option<storage_enums::PaymentMethod>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    customer_id: &Option<CustomerId>,
    billing_name: Option<masking::Secret<String>>,
) -> RouterResult<payment_methods::PaymentMethodCreate> {
    match payment_method_data {
        Some(pm_data) => match payment_method {
            Some(payment_method) => match pm_data {
                domain::PaymentMethodData::Card(card) => {
                    let card_detail = payment_methods::CardDetail {
                        card_number: card.card_number.clone(),
                        card_exp_month: card.card_exp_month.clone(),
                        card_exp_year: card.card_exp_year.clone(),
                        card_holder_name: billing_name,
                        nick_name: card.nick_name.clone(),
                        card_issuing_country: card.card_issuing_country.clone(),
                        card_network: card.card_network.clone(),
                        card_issuer: card.card_issuer.clone(),
                        card_type: card.card_type.clone(),
                    };
                    let payment_method_request = payment_methods::PaymentMethodCreate {
                        payment_method: Some(payment_method),
                        payment_method_type,
                        payment_method_issuer: card.card_issuer.clone(),
                        payment_method_issuer_code: None,
                        #[cfg(feature = "payouts")]
                        bank_transfer: None,
                        #[cfg(feature = "payouts")]
                        wallet: None,
                        card: Some(card_detail),
                        metadata: None,
                        customer_id: customer_id.clone(),
                        card_network: card
                            .card_network
                            .as_ref()
                            .map(|card_network| card_network.to_string()),
                        client_secret: None,
                        payment_method_data: None,
                        billing: None,
                        connector_mandate_details: None,
                        network_transaction_id: None,
                    };
                    Ok(payment_method_request)
                }
                _ => {
                    let payment_method_request = payment_methods::PaymentMethodCreate {
                        payment_method: Some(payment_method),
                        payment_method_type,
                        payment_method_issuer: None,
                        payment_method_issuer_code: None,
                        #[cfg(feature = "payouts")]
                        bank_transfer: None,
                        #[cfg(feature = "payouts")]
                        wallet: None,
                        card: None,
                        metadata: None,
                        customer_id: customer_id.clone(),
                        card_network: None,
                        client_secret: None,
                        payment_method_data: None,
                        billing: None,
                        connector_mandate_details: None,
                        network_transaction_id: None,
                    };

                    Ok(payment_method_request)
                }
            },
            None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_type"
            })
            .attach_printable("PaymentMethodType Required")),
        },
        None => Err(report!(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "payment_method_data"
        })
        .attach_printable("PaymentMethodData required Or Card is already saved")),
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn create_payment_method(
    state: &SessionState,
    req: api::PaymentMethodCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    cards::validate_and_vault_payment_method(state, req, merchant_account, key_store).await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn payment_method_intent_create(
    state: &SessionState,
    req: api::PaymentMethodIntentCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    cards::payment_method_intent_create(state, req, merchant_account, key_store).await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn payment_method_intent_confirm(
    state: &SessionState,
    req: api::PaymentMethodIntentConfirm,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    pm_id: String,
) -> errors::RouterResponse<api::PaymentMethodResponse> {
    cards::payment_method_intent_confirm(state, req, merchant_account, key_store, pm_id).await
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
pub trait VaultingInterface {
    fn get_vault_action_url() -> &'static str;
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
pub trait VaultingDataInterface {
    fn get_vaulting_data_key(&self) -> String;
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultFingerprintRequest {
    pub data: String,
    pub key: String,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct VaultFingerprintResponse {
    pub fingerprint_id: String,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVaultRequest<D> {
    pub entity_id: common_utils::id_type::MerchantId,
    pub vault_id: String,
    pub data: D,
    pub ttl: i64,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVaultResponse {
    pub entity_id: common_utils::id_type::MerchantId,
    pub vault_id: String,
    pub fingerprint_id: Option<String>,
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AddVault;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetVaultFingerprint;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingInterface for AddVault {
    fn get_vault_action_url() -> &'static str {
        ADD_VAULT_REQUEST_URL
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingInterface for GetVaultFingerprint {
    fn get_vault_action_url() -> &'static str {
        VAULT_FINGERPRINT_REQUEST_URL
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[async_trait::async_trait]
impl VaultingDataInterface for api::PaymentMethodCreateData {
    fn get_vaulting_data_key(&self) -> String {
        match &self {
            api::PaymentMethodCreateData::Card(card) => card.card_number.to_string(),
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub struct PaymentMethodClientSecret;

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl PaymentMethodClientSecret {
    pub fn generate(payment_method_id: &str) -> String {
        generate_id(
            consts::ID_LENGTH,
            format!("{payment_method_id}_secret").as_str(),
        )
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
async fn create_vault_request<R: VaultingInterface>(
    jwekey: &settings::Jwekey,
    locker: &settings::Locker,
    payload: Vec<u8>,
) -> errors::CustomResult<services::Request, errors::VaultError> {
    let private_key = jwekey.vault_private_key.peek().as_bytes();

    let jws = encryption::jws_sign_payload(&payload, &locker.locker_signing_key_id, private_key)
        .await
        .change_context(errors::VaultError::RequestEncryptionFailed)?;

    let jwe_payload = pm_transforms::create_jwe_body_for_vault(jwekey, &jws).await?;

    let mut url = locker.host.to_owned();
    url.push_str(R::get_vault_action_url());
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, VAULT_HEADER_CONTENT_TYPE.into());
    request.set_body(RequestContent::Json(Box::new(jwe_payload)));
    Ok(request)
}
