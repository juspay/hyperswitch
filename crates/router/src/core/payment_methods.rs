pub mod cards;
pub mod migration;
pub mod network_tokenization;
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
use common_utils::{consts::DEFAULT_LOCALE, id_type};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use common_utils::{
    crypto::{self, Encryptable},
    ext_traits::{AsyncExt, Encode, StringExt, ValueExt},
    fp_utils::when,
    generate_id,
    request::RequestContent,
    types as util_types,
};
use diesel_models::{
    enums, GenericLinkNew, PaymentMethodCollectLink, PaymentMethodCollectLinkData,
};
use error_stack::{report, ResultExt};
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use hyperswitch_domain_models::api::{GenericLinks, GenericLinksData};
use hyperswitch_domain_models::payments::{payment_attempt::PaymentAttempt, PaymentIntent};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use masking::ExposeInterface;
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};
use time::Duration;

use super::{
    errors::{RouterResponse, StorageErrorExt},
    pm_auth,
};
#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
use crate::{
    configs::settings,
    core::{payment_methods::transformers as pm_transforms, utils as core_utils},
    headers, logger,
    routes::payment_methods as pm_routes,
    services::encryption,
    types::{
        api::{self, payment_methods::PaymentMethodCreateExt},
        payment_methods as pm_types,
        storage::PaymentMethodListContext,
    },
    utils::ext_traits::OptionExt,
};
use crate::{
    consts,
    core::{
        errors::{self, RouterResult},
        payments::helpers as payment_helpers,
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

#[instrument(skip_all)]
pub async fn retrieve_payment_method_core(
    pm_data: &Option<domain::PaymentMethodData>,
    state: &SessionState,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    merchant_key_store: &domain::MerchantKeyStore,
    business_profile: Option<&domain::Profile>,
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
        pm_opt @ Some(pm @ domain::PaymentMethodData::BankDebit(_)) => {
            let payment_token = payment_helpers::store_payment_method_data_in_vault(
                state,
                payment_attempt,
                payment_intent,
                enums::PaymentMethod::BankDebit,
                pm,
                merchant_key_store,
                business_profile,
            )
            .await?;

            Ok((pm_opt.to_owned(), payment_token))
        }
        pm @ Some(domain::PaymentMethodData::PayLater(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::Crypto(_)) => Ok((pm.to_owned(), None)),
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
    let customer_id = id_type::CustomerId::try_from(Cow::from(pm_collect_link.primary_reference))
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
                let customer_id = id_type::CustomerId::try_from(Cow::from(
                    pm_collect_link.primary_reference.clone(),
                ))
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
                    publishable_key: Secret::new(merchant_account.publishable_key),
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn add_payment_method_status_update_task(
    db: &dyn StorageInterface,
    payment_method: &domain::PaymentMethod,
    prev_status: enums::PaymentMethodStatus,
    curr_status: enums::PaymentMethodStatus,
    merchant_id: &id_type::MerchantId,
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

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn retrieve_payment_method_with_token(
    _state: &SessionState,
    _merchant_key_store: &domain::MerchantKeyStore,
    _token_data: &storage::PaymentTokenData,
    _payment_intent: &PaymentIntent,
    _card_token_data: Option<&domain::CardToken>,
    _customer: &Option<domain::Customer>,
    _storage_scheme: common_enums::enums::MerchantStorageScheme,
    _mandate_id: Option<api_models::payments::MandateIds>,
    _payment_method_info: Option<domain::PaymentMethod>,
    _business_profile: &domain::Profile,
) -> RouterResult<storage::PaymentMethodDataWithId> {
    todo!()
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn retrieve_payment_method_with_token(
    state: &SessionState,
    merchant_key_store: &domain::MerchantKeyStore,
    token_data: &storage::PaymentTokenData,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    card_token_data: Option<&domain::CardToken>,
    customer: &Option<domain::Customer>,
    storage_scheme: common_enums::enums::MerchantStorageScheme,
    mandate_id: Option<api_models::payments::MandateIds>,
    payment_method_info: Option<domain::PaymentMethod>,
    business_profile: &domain::Profile,
) -> RouterResult<storage::PaymentMethodDataWithId> {
    let token = match token_data {
        storage::PaymentTokenData::TemporaryGeneric(generic_token) => {
            payment_helpers::retrieve_payment_method_with_temporary_token(
                state,
                &generic_token.token,
                payment_intent,
                payment_attempt,
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
                payment_attempt,
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
                mandate_id,
                payment_method_info,
                business_profile,
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
                mandate_id,
                payment_method_info,
                business_profile,
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
            pm_auth::retrieve_payment_method_from_auth_service(
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
    customer_id: &Option<id_type::CustomerId>,
    billing_name: Option<Secret<String>>,
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
    customer_id: &Option<id_type::CustomerId>,
    billing_name: Option<Secret<String>>,
    payment_method_billing_address: Option<&api_models::payments::Address>,
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
                        billing: payment_method_billing_address.cloned(),
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
) -> RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;

    let db = &*state.store;
    let merchant_id = merchant_account.get_id();
    let customer_id = req.customer_id.to_owned();

    db.find_customer_by_merchant_reference_id_merchant_id(
        &(state.into()),
        &customer_id,
        merchant_account.get_id(),
        key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;
    let key_manager_state = state.into();
    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| cards::create_encrypted_data(&key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    // create pm
    let payment_method_id = id_type::GlobalPaymentMethodId::generate("random_cell_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    let payment_method = create_payment_method_for_intent(
        state,
        req.metadata.clone(),
        &customer_id,
        payment_method_id,
        merchant_id,
        key_store,
        merchant_account.storage_scheme,
        payment_method_billing_address.map(Into::into),
    )
    .await
    .attach_printable("Failed to add Payment method to DB")?;

    let payment_method_data = pm_types::PaymentMethodVaultingData::from(req.payment_method_data);

    let vaulting_result = vault_payment_method(
        state,
        &payment_method_data,
        merchant_account,
        key_store,
        None,
    )
    .await;

    let response = match vaulting_result {
        Ok(resp) => {
            let pm_update = create_pm_additional_data_update(
                &payment_method_data,
                state,
                key_store,
                Some(resp.vault_id.get_string_repr().clone()),
                Some(req.payment_method),
                Some(req.payment_method_type),
            )
            .await
            .attach_printable("Unable to create Payment method data")?;

            let payment_method = db
                .update_payment_method(
                    &(state.into()),
                    key_store,
                    payment_method,
                    pm_update,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update payment method in db")?;

            let resp = pm_transforms::generate_payment_method_response(&payment_method)?;

            Ok(resp)
        }
        Err(e) => {
            let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                status: Some(enums::PaymentMethodStatus::Inactive),
            };

            db.update_payment_method(
                &(state.into()),
                key_store,
                payment_method,
                pm_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;

            Err(e)
        }
    }?;

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn payment_method_intent_create(
    state: &SessionState,
    req: api::PaymentMethodIntentCreate,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
) -> RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;
    let merchant_id = merchant_account.get_id();
    let customer_id = req.customer_id.to_owned();

    db.find_customer_by_merchant_reference_id_merchant_id(
        &(state.into()),
        &customer_id,
        merchant_account.get_id(),
        key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;
    let key_manager_state = state.into();
    let payment_method_billing_address: Option<Encryptable<Secret<serde_json::Value>>> = req
        .billing
        .clone()
        .async_map(|billing| cards::create_encrypted_data(&key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?;

    // create pm entry

    let payment_method_id = id_type::GlobalPaymentMethodId::generate("random_cell_id")
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    let payment_method = create_payment_method_for_intent(
        state,
        req.metadata.clone(),
        &customer_id,
        payment_method_id,
        merchant_id,
        key_store,
        merchant_account.storage_scheme,
        payment_method_billing_address.map(Into::into),
    )
    .await
    .attach_printable("Failed to add Payment method to DB")?;

    let resp = pm_transforms::generate_payment_method_response(&payment_method)?;

    Ok(services::ApplicationResponse::Json(resp))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn payment_method_intent_confirm(
    state: &SessionState,
    req: api::PaymentMethodIntentConfirm,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    pm_id: String,
) -> RouterResponse<api::PaymentMethodResponse> {
    req.validate()?;

    let db = &*state.store;
    let client_secret = req.client_secret.clone();
    let pm_id = id_type::GlobalPaymentMethodId::generate_from_string(pm_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    let payment_method = db
        .find_payment_method(
            &(state.into()),
            key_store,
            &pm_id,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    when(
        cards::authenticate_pm_client_secret_and_check_expiry(&client_secret, &payment_method)?,
        || Err(errors::ApiErrorResponse::ClientSecretExpired),
    )?;

    when(
        payment_method.status != enums::PaymentMethodStatus::AwaitingData,
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "Invalid pm_id provided: This Payment method cannot be confirmed"
                    .to_string(),
            })
        },
    )?;

    let customer_id = payment_method.customer_id.to_owned();
    db.find_customer_by_merchant_reference_id_merchant_id(
        &(state.into()),
        &customer_id,
        merchant_account.get_id(),
        key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let payment_method_data = pm_types::PaymentMethodVaultingData::from(req.payment_method_data);

    let vaulting_result = vault_payment_method(
        state,
        &payment_method_data,
        merchant_account,
        key_store,
        None,
    )
    .await;

    let response = match vaulting_result {
        Ok(resp) => {
            let pm_update = create_pm_additional_data_update(
                &payment_method_data,
                state,
                key_store,
                Some(resp.vault_id.get_string_repr().clone()),
                Some(req.payment_method),
                Some(req.payment_method_type),
            )
            .await
            .attach_printable("Unable to create Payment method data")?;

            let payment_method = db
                .update_payment_method(
                    &(state.into()),
                    key_store,
                    payment_method,
                    pm_update,
                    merchant_account.storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update payment method in db")?;

            let resp = pm_transforms::generate_payment_method_response(&payment_method)?;

            Ok(resp)
        }
        Err(e) => {
            let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                status: Some(enums::PaymentMethodStatus::Inactive),
            };

            db.update_payment_method(
                &(state.into()),
                key_store,
                payment_method,
                pm_update,
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;

            Err(e)
        }
    }?;

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method_in_db(
    state: &SessionState,
    req: &api::PaymentMethodCreate,
    customer_id: &id_type::CustomerId,
    payment_method_id: id_type::GlobalPaymentMethodId,
    locker_id: Option<domain::VaultId>,
    merchant_id: &id_type::MerchantId,
    customer_acceptance: Option<common_utils::pii::SecretSerdeValue>,
    payment_method_data: crypto::OptionalEncryptableValue,
    key_store: &domain::MerchantKeyStore,
    connector_mandate_details: Option<diesel_models::PaymentsMandateReference>,
    status: Option<enums::PaymentMethodStatus>,
    network_transaction_id: Option<String>,
    storage_scheme: enums::MerchantStorageScheme,
    payment_method_billing_address: crypto::OptionalEncryptableValue,
    card_scheme: Option<String>,
) -> errors::CustomResult<domain::PaymentMethod, errors::ApiErrorResponse> {
    let db = &*state.store;
    let client_secret = pm_types::PaymentMethodClientSecret::generate(&payment_method_id);
    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            &state.into(),
            key_store,
            domain::PaymentMethod {
                customer_id: customer_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                id: payment_method_id,
                locker_id,
                payment_method: Some(req.payment_method),
                payment_method_type: Some(req.payment_method_type),
                payment_method_data,
                connector_mandate_details,
                customer_acceptance,
                client_secret: Some(client_secret),
                status: status.unwrap_or(enums::PaymentMethodStatus::Active),
                network_transaction_id: network_transaction_id.to_owned(),
                created_at: current_time,
                last_modified: current_time,
                last_used_at: current_time,
                payment_method_billing_address,
                updated_by: None,
                version: domain::consts::API_VERSION,
                locker_fingerprint_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                network_token_requestor_reference_id: None,
            },
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    Ok(response)
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method_for_intent(
    state: &SessionState,
    metadata: Option<common_utils::pii::SecretSerdeValue>,
    customer_id: &id_type::CustomerId,
    payment_method_id: id_type::GlobalPaymentMethodId,
    merchant_id: &id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    payment_method_billing_address: crypto::OptionalEncryptableValue,
) -> errors::CustomResult<domain::PaymentMethod, errors::ApiErrorResponse> {
    let db = &*state.store;
    let client_secret = pm_types::PaymentMethodClientSecret::generate(&payment_method_id);
    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            &state.into(),
            key_store,
            domain::PaymentMethod {
                customer_id: customer_id.to_owned(),
                merchant_id: merchant_id.to_owned(),
                id: payment_method_id,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_data: None,
                connector_mandate_details: None,
                customer_acceptance: None,
                client_secret: Some(client_secret),
                status: enums::PaymentMethodStatus::AwaitingData,
                network_transaction_id: None,
                created_at: current_time,
                last_modified: current_time,
                last_used_at: current_time,
                payment_method_billing_address,
                updated_by: None,
                version: domain::consts::API_VERSION,
                locker_fingerprint_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                network_token_requestor_reference_id: None,
            },
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    Ok(response)
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn create_pm_additional_data_update(
    pmd: &pm_types::PaymentMethodVaultingData,
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    vault_id: Option<String>,
    payment_method_type: Option<api_enums::PaymentMethod>,
    payment_method_subtype: Option<api_enums::PaymentMethodType>,
) -> RouterResult<storage::PaymentMethodUpdate> {
    let card = match pmd {
        pm_types::PaymentMethodVaultingData::Card(card) => {
            api::PaymentMethodsData::Card(card.clone().into())
        }
    };
    let key_manager_state = state.into();
    let pmd: Encryptable<Secret<serde_json::Value>> =
        cards::create_encrypted_data(&key_manager_state, key_store, card)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to encrypt Payment method data")?;

    let pm_update = storage::PaymentMethodUpdate::AdditionalDataUpdate {
        status: Some(enums::PaymentMethodStatus::Active),
        locker_id: vault_id,
        payment_method_type_v2: payment_method_type,
        payment_method_subtype,
        payment_method_data: Some(pmd.into()),
        network_token_requestor_reference_id: None,
        network_token_locker_id: None,
        network_token_payment_method_data: None,
    };

    Ok(pm_update)
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn vault_payment_method(
    state: &SessionState,
    pmd: &pm_types::PaymentMethodVaultingData,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    existing_vault_id: Option<domain::VaultId>,
) -> RouterResult<pm_types::AddVaultResponse> {
    let db = &*state.store;

    // get fingerprint_id from vault
    let fingerprint_id_from_vault = vault::get_fingerprint_id_from_vault(state, pmd)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get fingerprint_id from vault")?;

    // throw back error if payment method is duplicated
    when(
        Some(
            db.find_payment_method_by_fingerprint_id(
                &(state.into()),
                key_store,
                &fingerprint_id_from_vault,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to find payment method by fingerprint_id")?,
        )
        .is_some(),
        || {
            Err(report!(errors::ApiErrorResponse::DuplicatePaymentMethod)
                .attach_printable("Cannot vault duplicate payment method"))
        },
    )?;

    let resp_from_vault =
        vault::add_payment_method_to_vault(state, merchant_account, pmd, existing_vault_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add payment method in vault")?;

    Ok(resp_from_vault)
}

#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
async fn get_pm_list_context(
    state: &SessionState,
    payment_method: &enums::PaymentMethod,
    _key_store: &domain::MerchantKeyStore,
    pm: &domain::PaymentMethod,
    _parent_payment_method_token: Option<String>,
    is_payment_associated: bool,
) -> Result<Option<PaymentMethodListContext>, error_stack::Report<errors::ApiErrorResponse>> {
    let payment_method_retrieval_context = match payment_method {
        enums::PaymentMethod::Card => {
            let card_details = cards::get_card_details_with_locker_fallback(pm, state).await?;

            card_details.as_ref().map(|card| PaymentMethodListContext {
                card_details: Some(card.clone()),
                #[cfg(feature = "payouts")]
                bank_transfer_details: None,
                hyperswitch_token_data: is_payment_associated.then_some(
                    storage::PaymentTokenData::permanent_card(
                        Some(pm.get_id().clone()),
                        pm.locker_id
                            .as_ref()
                            .map(|id| id.get_string_repr().clone())
                            .or(Some(pm.get_id().get_string_repr().to_owned())),
                        pm.locker_id
                            .as_ref()
                            .map(|id| id.get_string_repr().clone())
                            .unwrap_or(pm.get_id().get_string_repr().to_owned()),
                    ),
                ),
            })
        }

        enums::PaymentMethod::BankDebit => {
            // Retrieve the pm_auth connector details so that it can be tokenized
            let bank_account_token_data = cards::get_bank_account_connector_details(pm)
                .await
                .unwrap_or_else(|err| {
                    logger::error!(error=?err);
                    None
                });

            bank_account_token_data.map(|data| {
                let token_data = storage::PaymentTokenData::AuthBankDebit(data);

                PaymentMethodListContext {
                    card_details: None,
                    #[cfg(feature = "payouts")]
                    bank_transfer_details: None,
                    hyperswitch_token_data: is_payment_associated.then_some(token_data),
                }
            })
        }

        _ => Some(PaymentMethodListContext {
            card_details: None,
            #[cfg(feature = "payouts")]
            bank_transfer_details: None,
            hyperswitch_token_data: is_payment_associated.then_some(
                storage::PaymentTokenData::temporary_generic(generate_id(
                    consts::ID_LENGTH,
                    "token",
                )),
            ),
        }),
    };

    Ok(payment_method_retrieval_context)
}

#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
pub async fn list_customer_payment_method_util(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    req: Option<api::PaymentMethodListRequest>,
    customer_id: Option<id_type::CustomerId>,
    is_payment_associated: bool,
) -> RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let limit = req.as_ref().and_then(|pml_req| pml_req.limit);

    let (customer_id, payment_intent) = if is_payment_associated {
        let cloned_secret = req.and_then(|r| r.client_secret.clone());
        let payment_intent = payment_helpers::verify_payment_intent_time_and_client_secret(
            &state,
            &merchant_account,
            &key_store,
            cloned_secret,
        )
        .await?;

        (
            payment_intent
                .as_ref()
                .and_then(|pi| pi.customer_id.clone()),
            payment_intent,
        )
    } else {
        (customer_id, None)
    };

    let resp = if let Some(cust) = customer_id {
        Box::pin(list_customer_payment_method(
            &state,
            &merchant_account,
            profile,
            key_store,
            payment_intent,
            &cust,
            limit,
            is_payment_associated,
        ))
        .await?
    } else {
        let response = api::CustomerPaymentMethodsListResponse {
            customer_payment_methods: Vec::new(),
            is_guest_customer: Some(true),
        };
        services::ApplicationResponse::Json(response)
    };

    Ok(resp)
}

#[allow(clippy::too_many_arguments)]
#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
pub async fn list_customer_payment_method(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    profile: domain::Profile,
    key_store: domain::MerchantKeyStore,
    payment_intent: Option<PaymentIntent>,
    customer_id: &id_type::CustomerId,
    limit: Option<i64>,
    is_payment_associated: bool,
) -> RouterResponse<api::CustomerPaymentMethodsListResponse> {
    let db = &*state.store;
    let key_manager_state = &(state).into();

    let customer = db
        .find_customer_by_merchant_reference_id_merchant_id(
            key_manager_state,
            customer_id,
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let payments_info = payment_intent
        .async_map(|pi| {
            pm_types::SavedPMLPaymentsInfo::form_payments_info(
                pi,
                merchant_account,
                profile,
                db,
                key_manager_state,
                &key_store,
            )
        })
        .await
        .transpose()?;

    let saved_payment_methods = db
        .find_payment_method_by_customer_id_merchant_id_status(
            key_manager_state,
            &key_store,
            customer_id,
            merchant_account.get_id(),
            common_enums::PaymentMethodStatus::Active,
            limit,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let mut filtered_saved_payment_methods_ctx = Vec::new();
    for pm in saved_payment_methods.into_iter() {
        let payment_method = pm.payment_method.get_required_value("payment_method")?;
        let parent_payment_method_token =
            is_payment_associated.then(|| generate_id(consts::ID_LENGTH, "token"));

        let pm_list_context = get_pm_list_context(
            state,
            &payment_method,
            &key_store,
            &pm,
            parent_payment_method_token.clone(),
            is_payment_associated,
        )
        .await?;

        if let Some(ctx) = pm_list_context {
            filtered_saved_payment_methods_ctx.push((ctx, parent_payment_method_token, pm));
        }
    }

    let merchant_connector_accounts = if filtered_saved_payment_methods_ctx.iter().any(
        |(_pm_list_context, _parent_payment_method_token, pm)| {
            pm.connector_mandate_details.is_some()
        },
    ) {
        db.find_merchant_connector_account_by_merchant_id_and_disabled_list(
            key_manager_state,
            merchant_account.get_id(),
            true,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantAccountNotFound)?
    } else {
        Vec::new()
    };
    let merchant_connector_accounts =
        domain::MerchantConnectorAccounts::new(merchant_connector_accounts);

    let pm_list_futures = filtered_saved_payment_methods_ctx
        .into_iter()
        .map(|ctx| {
            generate_saved_pm_response(
                state,
                &key_store,
                merchant_account,
                &merchant_connector_accounts,
                ctx,
                &customer,
                payments_info.as_ref(),
            )
        })
        .collect::<Vec<_>>();

    let customer_pms = futures::future::join_all(pm_list_futures)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .attach_printable("Failed to obtain customer payment methods")?;

    let mut response = api::CustomerPaymentMethodsListResponse {
        customer_payment_methods: customer_pms,
        is_guest_customer: is_payment_associated.then_some(false), //to return this key only when the request is tied to a payment intent
    };

    if is_payment_associated {
        Box::pin(cards::perform_surcharge_ops(
            payments_info.as_ref().map(|pi| pi.payment_intent.clone()),
            state,
            merchant_account,
            key_store,
            payments_info.map(|pi| pi.profile),
            &mut response,
        ))
        .await?;
    }

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
async fn generate_saved_pm_response(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_accounts: &domain::MerchantConnectorAccounts,
    (pm_list_context, parent_payment_method_token, pm): (
        PaymentMethodListContext,
        Option<String>,
        domain::PaymentMethod,
    ),
    customer: &domain::Customer,
    payment_info: Option<&pm_types::SavedPMLPaymentsInfo>,
) -> Result<api::CustomerPaymentMethod, error_stack::Report<errors::ApiErrorResponse>> {
    let payment_method = pm.payment_method.get_required_value("payment_method")?;

    let bank_details = if payment_method == enums::PaymentMethod::BankDebit {
        cards::get_masked_bank_details(&pm)
            .await
            .unwrap_or_else(|err| {
                logger::error!(error=?err);
                None
            })
    } else {
        None
    };

    let payment_method_billing = pm
        .payment_method_billing_address
        .clone()
        .map(|decrypted_data| decrypted_data.into_inner().expose())
        .map(|decrypted_value| decrypted_value.parse_value("payment_method_billing_address"))
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("unable to parse payment method billing address details")?;

    let (is_connector_agnostic_mit_enabled, requires_cvv, off_session_payment_flag, profile_id) =
        payment_info
            .map(|pi| {
                (
                    pi.is_connector_agnostic_mit_enabled,
                    pi.collect_cvv_during_payment,
                    pi.off_session_payment_flag,
                    Some(pi.profile.get_id().to_owned()),
                )
            })
            .unwrap_or((false, false, false, Default::default()));

    let mca_enabled = cards::get_mca_status(
        state,
        key_store,
        profile_id,
        merchant_account.get_id(),
        is_connector_agnostic_mit_enabled,
        pm.connector_mandate_details.as_ref(),
        pm.network_transaction_id.as_ref(),
        merchant_connector_accounts,
    )
    .await;

    let requires_cvv = if is_connector_agnostic_mit_enabled {
        requires_cvv
            && !(off_session_payment_flag
                && (pm.connector_mandate_details.is_some() || pm.network_transaction_id.is_some()))
    } else {
        requires_cvv && !(off_session_payment_flag && pm.connector_mandate_details.is_some())
    };

    let pmd = if let Some(card) = pm_list_context.card_details.as_ref() {
        Some(api::PaymentMethodListData::Card(card.clone()))
    } else if cfg!(feature = "payouts") {
        pm_list_context
            .bank_transfer_details
            .clone()
            .map(api::PaymentMethodListData::Bank)
    } else {
        None
    };

    let pma = api::CustomerPaymentMethod {
        payment_token: parent_payment_method_token.clone(),
        payment_method_id: pm.get_id().get_string_repr().to_owned(),
        customer_id: pm.customer_id.to_owned(),
        payment_method,
        payment_method_type: pm.payment_method_type,
        payment_method_data: pmd,
        recurring_enabled: mca_enabled,
        created: Some(pm.created_at),
        bank: bank_details,
        surcharge_details: None,
        requires_cvv: requires_cvv
            && !(off_session_payment_flag && pm.connector_mandate_details.is_some()),
        last_used_at: Some(pm.last_used_at),
        is_default: customer
            .default_payment_method_id
            .as_ref()
            .is_some_and(|payment_method_id| payment_method_id == pm.get_id().get_string_repr()),
        billing: payment_method_billing,
    };

    payment_info
        .async_map(|pi| {
            pi.perform_payment_ops(state, parent_payment_method_token, &pma, pm_list_context)
        })
        .await
        .transpose()?;

    Ok(pma)
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: SessionState,
    pm: api::PaymentMethodId,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<api::PaymentMethodResponse> {
    let db = state.store.as_ref();
    let pm_id = id_type::GlobalPaymentMethodId::generate_from_string(pm.payment_method_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    let payment_method = db
        .find_payment_method(
            &((&state).into()),
            &key_store,
            &pm_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let pmd = payment_method
        .payment_method_data
        .clone()
        .map(|x| x.into_inner().expose())
        .and_then(|v| serde_json::from_value::<api::PaymentMethodsData>(v).ok())
        .and_then(|pmd| match pmd {
            api::PaymentMethodsData::Card(card) => {
                Some(api::PaymentMethodResponseData::Card(card.into()))
            }
            _ => None,
        });

    let resp = api::PaymentMethodResponse {
        merchant_id: payment_method.merchant_id.to_owned(),
        customer_id: payment_method.customer_id.to_owned(),
        payment_method_id: payment_method.id.get_string_repr().to_string(),
        payment_method: payment_method.payment_method,
        payment_method_type: payment_method.payment_method_type,
        created: Some(payment_method.created_at),
        recurring_enabled: false,
        last_used_at: Some(payment_method.last_used_at),
        client_secret: payment_method.client_secret.clone(),
        payment_method_data: pmd,
    };

    Ok(services::ApplicationResponse::Json(resp))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn update_payment_method(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    req: api::PaymentMethodUpdate,
    payment_method_id: &str,
    key_store: domain::MerchantKeyStore,
) -> RouterResponse<api::PaymentMethodResponse> {
    let db = state.store.as_ref();

    let pm_id = id_type::GlobalPaymentMethodId::generate_from_string(payment_method_id.to_string())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    let payment_method = db
        .find_payment_method(
            &((&state).into()),
            &key_store,
            &pm_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    let current_vault_id = payment_method.locker_id.clone();

    when(
        payment_method.status == enums::PaymentMethodStatus::AwaitingData,
        || {
            Err(errors::ApiErrorResponse::InvalidRequestData {
                message: "This Payment method is awaiting data and hence cannot be updated"
                    .to_string(),
            })
        },
    )?;

    let pmd: pm_types::PaymentMethodVaultingData =
        vault::retrieve_payment_method_from_vault(&state, &merchant_account, &payment_method)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to retrieve payment method from vault")?
            .data
            .expose()
            .parse_struct("PaymentMethodVaultingData")
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to parse PaymentMethodVaultingData")?;

    let vault_request_data =
        pm_transforms::generate_pm_vaulting_req_from_update_request(pmd, req.payment_method_data);

    let vaulting_response = vault_payment_method(
        &state,
        &vault_request_data,
        &merchant_account,
        &key_store,
        current_vault_id, // using current vault_id for now, will have to refactor this
    ) // to generate new one on each vaulting later on
    .await
    .attach_printable("Failed to add payment method in vault")?;

    let pm_update = create_pm_additional_data_update(
        &vault_request_data,
        &state,
        &key_store,
        Some(vaulting_response.vault_id.get_string_repr().clone()),
        payment_method.payment_method,
        payment_method.payment_method_type,
    )
    .await
    .attach_printable("Unable to create Payment method data")?;

    let payment_method = db
        .update_payment_method(
            &((&state).into()),
            &key_store,
            payment_method,
            pm_update,
            merchant_account.storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update payment method in db")?;

    let response = pm_transforms::generate_payment_method_response(&payment_method)?;

    // Add a PT task to handle payment_method delete from vault

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: SessionState,
    pm_id: api::PaymentMethodId,
    key_store: domain::MerchantKeyStore,
    merchant_account: domain::MerchantAccount,
) -> RouterResponse<api::PaymentMethodDeleteResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let pm_id = id_type::GlobalPaymentMethodId::generate_from_string(pm_id.payment_method_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    let payment_method = db
        .find_payment_method(
            &((&state).into()),
            &key_store,
            &pm_id,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let vault_id = payment_method
        .locker_id
        .clone()
        .get_required_value("locker_id")
        .attach_printable("Missing locker_id in PaymentMethod")?;

    let _customer = db
        .find_customer_by_global_id(
            key_manager_state,
            payment_method.customer_id.get_string_repr(),
            merchant_account.get_id(),
            &key_store,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Customer not found for the payment method")?;

    // Soft delete
    let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
        status: Some(enums::PaymentMethodStatus::Inactive),
    };
    db.update_payment_method(
        &((&state).into()),
        &key_store,
        payment_method,
        pm_update,
        merchant_account.storage_scheme,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update payment method in db")?;

    vault::delete_payment_method_data_from_vault(&state, &merchant_account, vault_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete payment method from vault")?;

    let response = api::PaymentMethodDeleteResponse {
        payment_method_id: pm_id.get_string_repr().to_string(),
    };

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
impl pm_types::SavedPMLPaymentsInfo {
    pub async fn form_payments_info(
        payment_intent: PaymentIntent,
        merchant_account: &domain::MerchantAccount,
        profile: domain::Profile,
        db: &dyn StorageInterface,
        key_manager_state: &util_types::keymanager::KeyManagerState,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self> {
        let collect_cvv_during_payment = profile.should_collect_cvv_during_payment;

        let off_session_payment_flag = matches!(
            payment_intent.setup_future_usage,
            common_enums::FutureUsage::OffSession
        );

        let is_connector_agnostic_mit_enabled =
            profile.is_connector_agnostic_mit_enabled.unwrap_or(false);

        Ok(Self {
            payment_intent,
            profile,
            collect_cvv_during_payment,
            off_session_payment_flag,
            is_connector_agnostic_mit_enabled,
        })
    }

    pub async fn perform_payment_ops(
        &self,
        state: &SessionState,
        parent_payment_method_token: Option<String>,
        pma: &api::CustomerPaymentMethod,
        pm_list_context: PaymentMethodListContext,
    ) -> RouterResult<()> {
        let token = parent_payment_method_token
            .as_ref()
            .get_required_value("parent_payment_method_token")?;
        let hyperswitch_token_data = pm_list_context
            .hyperswitch_token_data
            .get_required_value("PaymentTokenData")?;

        let intent_fulfillment_time = self
            .profile
            .get_order_fulfillment_time()
            .unwrap_or(common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME);

        pm_routes::ParentPaymentMethodToken::create_key_for_token((token, pma.payment_method))
            .insert(intent_fulfillment_time, hyperswitch_token_data, state)
            .await?;

        Ok(())
    }
}
