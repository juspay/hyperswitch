pub mod cards;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
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
#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
use hyperswitch_domain_models::mandates::CommonMandateReference;
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
        domain::types as domain_types,
        payment_methods as pm_types,
        storage::{ephemeral_key, PaymentMethodListContext},
        transformers::{ForeignFrom, ForeignTryFrom},
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
        pm @ Some(domain::PaymentMethodData::MobilePayment(_)) => Ok((pm.to_owned(), None)),
        pm @ Some(domain::PaymentMethodData::NetworkToken(_)) => Ok((pm.to_owned(), None)),
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
    runner: storage::ProcessTrackerRunner,
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
        runner,
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
                payment_attempt.connector.clone(),
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
                payment_attempt.connector.clone(),
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
    payment_method_type: Option<storage_enums::PaymentMethod>,
    payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    customer_id: &Option<id_type::GlobalCustomerId>,
    billing_name: Option<Secret<String>>,
) -> RouterResult<payment_methods::PaymentMethodCreate> {
    match payment_method_data {
        Some(pm_data) => match payment_method_type {
            Some(payment_method_type) => match pm_data {
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
                        payment_method_type,
                        payment_method_subtype: payment_method_subtype
                            .get_required_value("payment_method_subtype")
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
    payment_method_billing_address: Option<&hyperswitch_domain_models::address::Address>,
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
                        //TODO: why are we using api model in router internally
                        billing: payment_method_billing_address.cloned().map(From::from),
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
    use common_utils::ext_traits::ValueExt;

    req.validate()?;

    let db = &*state.store;
    let merchant_id = merchant_account.get_id();
    let customer_id = req.customer_id.to_owned();
    let key_manager_state = &(state).into();

    db.find_customer_by_global_id(
        key_manager_state,
        &customer_id,
        merchant_account.get_id(),
        key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
    .attach_printable("Customer not found for the payment method")?;

    let payment_method_billing_address = req
        .billing
        .clone()
        .async_map(|billing| cards::create_encrypted_data(key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?
        .map(|encoded_address| {
            encoded_address.deserialize_inner_value(|value| value.parse_value("Address"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse Payment method billing address")?;

    // create pm
    let payment_method_id =
        id_type::GlobalPaymentMethodId::generate(&state.conf.cell_information.id)
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
        Ok((vaulting_resp, fingerprint_id)) => {
            let pm_update = create_pm_additional_data_update(
                &payment_method_data,
                state,
                key_store,
                Some(vaulting_resp.vault_id.get_string_repr().clone()),
                Some(req.payment_method_type),
                Some(req.payment_method_subtype),
                Some(fingerprint_id),
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
    let key_manager_state = &(state).into();

    db.find_customer_by_global_id(
        key_manager_state,
        &customer_id,
        merchant_account.get_id(),
        key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
    .attach_printable("Customer not found for the payment method")?;

    let payment_method_billing_address = req
        .billing
        .clone()
        .async_map(|billing| cards::create_encrypted_data(key_manager_state, key_store, billing))
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?
        .map(|encoded_address| {
            encoded_address.deserialize_inner_value(|value| value.parse_value("Address"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse Payment method billing address")?;

    // create pm entry

    let payment_method_id =
        id_type::GlobalPaymentMethodId::generate(&state.conf.cell_information.id)
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

#[cfg(feature = "v2")]
trait PerformFilteringOnEnabledPaymentMethods {
    fn perform_filtering(self) -> FilteredPaymentMethodsEnabled;
}

#[cfg(feature = "v2")]
impl PerformFilteringOnEnabledPaymentMethods
    for hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled
{
    fn perform_filtering(self) -> FilteredPaymentMethodsEnabled {
        FilteredPaymentMethodsEnabled(self.payment_methods_enabled)
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn list_payment_methods_for_session(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    profile: domain::Profile,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
) -> RouterResponse<api::PaymentMethodListResponse> {
    let key_manager_state = &(&state).into();

    let db = &*state.store;

    let payment_method_session = db
        .get_payment_methods_session(key_manager_state, &key_store, &payment_method_session_id)
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    let payment_connector_accounts = db
        .list_enabled_connector_accounts_by_profile_id(
            key_manager_state,
            profile.get_id(),
            &key_store,
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when fetching merchant connector accounts")?;

    let customer_payment_methods = list_customer_payment_method_core(
        &state,
        &merchant_account,
        &key_store,
        &payment_method_session.customer_id,
    )
    .await?;

    let response =
        hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled::from_payment_connectors_list(payment_connector_accounts)
            .perform_filtering()
            .get_required_fields(RequiredFieldsInput::new(state.conf.required_fields.clone()))
            .generate_response(customer_payment_methods.customer_payment_methods);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

#[cfg(feature = "v2")]
/// Container for the inputs required for the required fields
struct RequiredFieldsInput {
    required_fields_config: settings::RequiredFields,
}

#[cfg(feature = "v2")]
impl RequiredFieldsInput {
    fn new(required_fields_config: settings::RequiredFields) -> Self {
        Self {
            required_fields_config,
        }
    }
}

#[cfg(feature = "v2")]
/// Container for the filtered payment methods
struct FilteredPaymentMethodsEnabled(
    Vec<hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector>,
);

#[cfg(feature = "v2")]
trait GetRequiredFields {
    fn get_required_fields(
        &self,
        payment_method_enabled: &hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
    ) -> Option<&settings::RequiredFieldFinal>;
}

#[cfg(feature = "v2")]
impl GetRequiredFields for settings::RequiredFields {
    fn get_required_fields(
        &self,
        payment_method_enabled: &hyperswitch_domain_models::merchant_connector_account::PaymentMethodsEnabledForConnector,
    ) -> Option<&settings::RequiredFieldFinal> {
        self.0
            .get(&payment_method_enabled.payment_method)
            .and_then(|required_fields_for_payment_method| {
                required_fields_for_payment_method.0.get(
                    &payment_method_enabled
                        .payment_methods_enabled
                        .payment_method_subtype,
                )
            })
            .map(|connector_fields| &connector_fields.fields)
            .and_then(|connector_hashmap| connector_hashmap.get(&payment_method_enabled.connector))
    }
}

#[cfg(feature = "v2")]
impl FilteredPaymentMethodsEnabled {
    fn get_required_fields(
        self,
        input: RequiredFieldsInput,
    ) -> RequiredFieldsForEnabledPaymentMethodTypes {
        let required_fields_config = input.required_fields_config;

        let required_fields_info = self
            .0
            .into_iter()
            .map(|payment_methods_enabled| {
                let required_fields =
                    required_fields_config.get_required_fields(&payment_methods_enabled);

                let required_fields = required_fields
                    .map(|required_fields| {
                        let common_required_fields = required_fields
                            .common
                            .iter()
                            .flatten()
                            .map(ToOwned::to_owned);

                        // Collect mandate required fields because this is for zero auth mandates only
                        let mandate_required_fields = required_fields
                            .mandate
                            .iter()
                            .flatten()
                            .map(ToOwned::to_owned);

                        // Combine both common and mandate required fields
                        common_required_fields
                            .chain(mandate_required_fields)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                RequiredFieldsForEnabledPaymentMethod {
                    required_fields,
                    payment_method_type: payment_methods_enabled.payment_method,
                    payment_method_subtype: payment_methods_enabled
                        .payment_methods_enabled
                        .payment_method_subtype,
                }
            })
            .collect();

        RequiredFieldsForEnabledPaymentMethodTypes(required_fields_info)
    }
}

#[cfg(feature = "v2")]
/// Element container to hold the filtered payment methods with required fields
struct RequiredFieldsForEnabledPaymentMethod {
    required_fields: Vec<payment_methods::RequiredFieldInfo>,
    payment_method_subtype: common_enums::PaymentMethodType,
    payment_method_type: common_enums::PaymentMethod,
}

#[cfg(feature = "v2")]
/// Container to hold the filtered payment methods enabled with required fields
struct RequiredFieldsForEnabledPaymentMethodTypes(Vec<RequiredFieldsForEnabledPaymentMethod>);

#[cfg(feature = "v2")]
impl RequiredFieldsForEnabledPaymentMethodTypes {
    fn generate_response(
        self,
        customer_payment_methods: Vec<payment_methods::CustomerPaymentMethod>,
    ) -> payment_methods::PaymentMethodListResponse {
        let response_payment_methods = self
            .0
            .into_iter()
            .map(
                |payment_methods_enabled| payment_methods::ResponsePaymentMethodTypes {
                    payment_method_type: payment_methods_enabled.payment_method_type,
                    payment_method_subtype: payment_methods_enabled.payment_method_subtype,
                    required_fields: payment_methods_enabled.required_fields,
                    extra_information: None,
                },
            )
            .collect();

        payment_methods::PaymentMethodListResponse {
            payment_methods_enabled: response_payment_methods,
            customer_payment_methods,
        }
    }
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method_for_intent(
    state: &SessionState,
    metadata: Option<common_utils::pii::SecretSerdeValue>,
    customer_id: &id_type::GlobalCustomerId,
    payment_method_id: id_type::GlobalPaymentMethodId,
    merchant_id: &id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    payment_method_billing_address: Option<
        Encryptable<hyperswitch_domain_models::address::Address>,
    >,
) -> errors::CustomResult<domain::PaymentMethod, errors::ApiErrorResponse> {
    let db = &*state.store;

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
                payment_method_type: None,
                payment_method_subtype: None,
                payment_method_data: None,
                connector_mandate_details: None,
                customer_acceptance: None,
                client_secret: None,
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
    vault_fingerprint_id: Option<String>,
) -> RouterResult<storage::PaymentMethodUpdate> {
    let card = match pmd {
        pm_types::PaymentMethodVaultingData::Card(card) => {
            api::PaymentMethodsData::Card(card.clone().into())
        }
    };
    let key_manager_state = &(state).into();
    let pmd: Encryptable<Secret<serde_json::Value>> =
        cards::create_encrypted_data(key_manager_state, key_store, card)
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
        locker_fingerprint_id: vault_fingerprint_id,
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
) -> RouterResult<(pm_types::AddVaultResponse, String)> {
    let db = &*state.store;

    // get fingerprint_id from vault
    let fingerprint_id_from_vault = vault::get_fingerprint_id_from_vault(state, pmd)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get fingerprint_id from vault")?;

    // throw back error if payment method is duplicated
    when(
        db.find_payment_method_by_fingerprint_id(
            &(state.into()),
            key_store,
            &fingerprint_id_from_vault,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to find payment method by fingerprint_id")
        .inspect_err(|e| logger::error!("Vault Fingerprint_id error: {:?}", e))
        .is_ok(),
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

    Ok((resp_from_vault, fingerprint_id_from_vault))
}

// TODO: check if this function will be used for listing the customer payment methods for payments
#[allow(unused)]
#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
fn get_pm_list_context(
    payment_method_type: enums::PaymentMethod,
    payment_method: &domain::PaymentMethod,
    is_payment_associated: bool,
) -> Result<Option<PaymentMethodListContext>, error_stack::Report<errors::ApiErrorResponse>> {
    let payment_method_data = payment_method
        .payment_method_data
        .clone()
        .map(|payment_method_data| payment_method_data.into_inner());

    let payment_method_retrieval_context = match payment_method_data {
        Some(payment_methods::PaymentMethodsData::Card(card)) => {
            Some(PaymentMethodListContext::Card {
                card_details: api::CardDetailFromLocker::from(card),
                token_data: is_payment_associated.then_some(
                    storage::PaymentTokenData::permanent_card(
                        Some(payment_method.get_id().clone()),
                        payment_method
                            .locker_id
                            .as_ref()
                            .map(|id| id.get_string_repr().to_owned())
                            .or_else(|| Some(payment_method.get_id().get_string_repr().to_owned())),
                        payment_method
                            .locker_id
                            .as_ref()
                            .map(|id| id.get_string_repr().to_owned())
                            .unwrap_or_else(|| {
                                payment_method.get_id().get_string_repr().to_owned()
                            }),
                    ),
                ),
            })
        }
        Some(payment_methods::PaymentMethodsData::BankDetails(bank_details)) => {
            let get_bank_account_token_data =
                || -> errors::CustomResult<payment_methods::BankAccountTokenData, errors::ApiErrorResponse> {
                    let connector_details = bank_details
                        .connector_details
                        .first()
                        .cloned()
                        .ok_or(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to obtain bank account connector details")?;

                    let payment_method_subtype = payment_method
                        .get_payment_method_subtype()
                        .get_required_value("payment_method_subtype")
                        .attach_printable("PaymentMethodType not found")?;

                    Ok(payment_methods::BankAccountTokenData {
                        payment_method_type: payment_method_subtype,
                        payment_method: payment_method_type,
                        connector_details,
                    })
                };

            // Retrieve the pm_auth connector details so that it can be tokenized
            let bank_account_token_data = get_bank_account_token_data()
                .inspect_err(|error| logger::error!(?error))
                .ok();
            bank_account_token_data.map(|data| {
                let token_data = storage::PaymentTokenData::AuthBankDebit(data);

                PaymentMethodListContext::Bank {
                    token_data: is_payment_associated.then_some(token_data),
                }
            })
        }
        Some(payment_methods::PaymentMethodsData::WalletDetails(_)) | None => {
            Some(PaymentMethodListContext::TemporaryToken {
                token_data: is_payment_associated.then_some(
                    storage::PaymentTokenData::temporary_generic(generate_id(
                        consts::ID_LENGTH,
                        "token",
                    )),
                ),
            })
        }
    };

    Ok(payment_method_retrieval_context)
}

#[cfg(all(
    feature = "v2",
    feature = "payment_methods_v2",
    feature = "customer_v2"
))]
pub async fn list_customer_payment_method_core(
    state: &SessionState,
    merchant_account: &domain::MerchantAccount,
    key_store: &domain::MerchantKeyStore,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<api::CustomerPaymentMethodsListResponse> {
    let db = &*state.store;
    let key_manager_state = &(state).into();

    let saved_payment_methods = db
        .find_payment_method_by_global_customer_id_merchant_id_status(
            key_manager_state,
            key_store,
            customer_id,
            merchant_account.get_id(),
            common_enums::PaymentMethodStatus::Active,
            None,
            merchant_account.storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let customer_payment_methods = saved_payment_methods
        .into_iter()
        .map(ForeignTryFrom::foreign_try_from)
        .collect::<Result<Vec<api::CustomerPaymentMethod>, _>>()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = api::CustomerPaymentMethodsListResponse {
        customer_payment_methods,
    };

    Ok(response)
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

    when(
        payment_method.status == enums::PaymentMethodStatus::Inactive,
        || Err(errors::ApiErrorResponse::PaymentMethodNotFound),
    )?;

    let pmd = payment_method
        .payment_method_data
        .clone()
        .map(|x| x.into_inner())
        .and_then(|pmd| match pmd {
            api::PaymentMethodsData::Card(card) => {
                Some(api::PaymentMethodResponseData::Card(card.into()))
            }
            _ => None,
        });

    let resp = api::PaymentMethodResponse {
        merchant_id: payment_method.merchant_id.to_owned(),
        customer_id: payment_method.customer_id.to_owned(),
        id: payment_method.id.to_owned(),
        payment_method_type: payment_method.get_payment_method_type(),
        payment_method_subtype: payment_method.get_payment_method_subtype(),
        created: Some(payment_method.created_at),
        recurring_enabled: false,
        last_used_at: Some(payment_method.last_used_at),
        payment_method_data: pmd,
    };

    Ok(services::ApplicationResponse::Json(resp))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn update_payment_method(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: api::PaymentMethodUpdate,
    payment_method_id: &id_type::GlobalPaymentMethodId,
) -> RouterResponse<api::PaymentMethodResponse> {
    let response =
        update_payment_method_core(state, merchant_account, key_store, req, payment_method_id)
            .await?;

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
#[instrument(skip_all)]
pub async fn update_payment_method_core(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    req: api::PaymentMethodUpdate,
    payment_method_id: &id_type::GlobalPaymentMethodId,
) -> RouterResult<api::PaymentMethodResponse> {
    let db = state.store.as_ref();

    let payment_method = db
        .find_payment_method(
            &((&state).into()),
            &key_store,
            payment_method_id,
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
            .data;

    let vault_request_data =
        pm_transforms::generate_pm_vaulting_req_from_update_request(pmd, req.payment_method_data);

    let (vaulting_response, fingerprint_id) = vault_payment_method(
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
        payment_method.get_payment_method_type(),
        payment_method.get_payment_method_subtype(),
        Some(fingerprint_id),
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

    Ok(response)
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

    when(
        payment_method.status == enums::PaymentMethodStatus::Inactive,
        || Err(errors::ApiErrorResponse::PaymentMethodNotFound),
    )?;

    let vault_id = payment_method
        .locker_id
        .clone()
        .get_required_value("locker_id")
        .attach_printable("Missing locker_id in PaymentMethod")?;

    let _customer = db
        .find_customer_by_global_id(
            key_manager_state,
            &payment_method.customer_id,
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

    let response = api::PaymentMethodDeleteResponse { id: pm_id };

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
trait EncryptableData {
    type Output;
    async fn encrypt_data(
        &self,
        key_manager_state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self::Output>;
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl EncryptableData for payment_methods::PaymentMethodSessionRequest {
    type Output = hyperswitch_domain_models::payment_methods::DecryptedPaymentMethodsSession;

    async fn encrypt_data(
        &self,
        key_manager_state: &common_utils::types::keymanager::KeyManagerState,
        key_store: &domain::MerchantKeyStore,
    ) -> RouterResult<Self::Output> {
        use common_utils::types::keymanager::ToEncryptable;

        let encrypted_billing_address = self
            .billing
            .clone()
            .map(|address| address.encode_to_value())
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encode billing address")?
            .map(Secret::new);

        let batch_encrypted_data = domain_types::crypto_operation(
            key_manager_state,
            common_utils::type_name!(hyperswitch_domain_models::payment_methods::PaymentMethodsSession),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodsSession::to_encryptable(
                    hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodsSession {
                       billing: encrypted_billing_address,
                    },
                ),
            ),
            common_utils::types::keymanager::Identifier::Merchant(key_store.merchant_id.clone()),
            key_store.key.peek(),
        )
        .await
        .and_then(|val| val.try_into_batchoperation())
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment methods session details".to_string())?;

        let encrypted_data =
        hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodsSession::from_encryptable(
            batch_encrypted_data,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment methods session detailss")?;

        Ok(encrypted_data)
    }
}

#[cfg(feature = "v2")]
pub async fn payment_methods_session_create(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: payment_methods::PaymentMethodSessionRequest,
) -> RouterResponse<payment_methods::PaymentMethodsSessionResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    db.find_customer_by_global_id(
        key_manager_state,
        &request.customer_id,
        merchant_account.get_id(),
        &key_store,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)?;

    let payment_methods_session_id =
        id_type::GlobalPaymentMethodSessionId::generate(&state.conf.cell_information.id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to generate GlobalPaymentMethodSessionId")?;

    let encrypted_data = request
        .encrypt_data(key_manager_state, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to encrypt payment methods session data")?;

    let billing = encrypted_data
        .billing
        .as_ref()
        .map(|data| {
            data.clone()
                .deserialize_inner_value(|value| value.parse_value("Address"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to decode billing address")?;

    // If not passed in the request, use the default value from constants
    let expires_in = request
        .expires_in
        .unwrap_or(consts::DEFAULT_PAYMENT_METHOD_SESSION_EXPIRY)
        .into();

    let expires_at = common_utils::date_time::now().saturating_add(Duration::seconds(expires_in));

    let client_secret = payment_helpers::create_client_secret(
        &state,
        merchant_account.get_id(),
        util_types::authentication::ResourceId::PaymentMethodSession(
            payment_methods_session_id.clone(),
        ),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to create client secret")?;

    let payment_method_session_domain_model =
        hyperswitch_domain_models::payment_methods::PaymentMethodsSession {
            id: payment_methods_session_id,
            customer_id: request.customer_id,
            billing,
            psp_tokenization: request.psp_tokenization,
            network_tokenization: request.network_tokenization,
            expires_at,
        };

    db.insert_payment_methods_session(
        key_manager_state,
        &key_store,
        payment_method_session_domain_model.clone(),
        expires_in,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to insert payment methods session in db")?;

    let response = payment_methods::PaymentMethodsSessionResponse::foreign_from((
        payment_method_session_domain_model,
        client_secret.secret,
    ));

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn payment_methods_session_retrieve(
    state: SessionState,
    _merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
) -> RouterResponse<payment_methods::PaymentMethodsSessionResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let payment_method_session_domain_model = db
        .get_payment_methods_session(key_manager_state, &key_store, &payment_method_session_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to retrieve payment methods session from db")?;

    let response = payment_methods::PaymentMethodsSessionResponse::foreign_from((
        payment_method_session_domain_model,
        Secret::new("CLIENT_SECRET_REDACTED".to_string()),
    ));

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn payment_methods_session_update_payment_method(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
    request: payment_methods::PaymentMethodSessionUpdateSavedPaymentMethod,
) -> RouterResponse<payment_methods::PaymentMethodResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    // Validate if the session still exists
    db.get_payment_methods_session(key_manager_state, &key_store, &payment_method_session_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to retrieve payment methods session from db")?;

    let payment_method_update_request = request.payment_method_update_request;

    let updated_payment_method = update_payment_method_core(
        state,
        merchant_account,
        key_store,
        payment_method_update_request,
        &request.payment_method_id,
    )
    .await
    .attach_printable("Failed to update saved payment method")?;

    Ok(services::ApplicationResponse::Json(updated_payment_method))
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
        let token_data = pm_list_context
            .get_token_data()
            .get_required_value("PaymentTokenData")?;

        let intent_fulfillment_time = self
            .profile
            .get_order_fulfillment_time()
            .unwrap_or(common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME);

        pm_routes::ParentPaymentMethodToken::create_key_for_token((token, pma.payment_method_type))
            .insert(intent_fulfillment_time, token_data, state)
            .await?;

        Ok(())
    }
}
