pub mod access_token;
#[cfg(feature = "v1")]
pub mod batch_retrieve;
pub mod cards;
pub mod migration;
pub mod network_tokenization;
pub mod surcharge_decision_configs;
#[cfg(feature = "v1")]
pub mod tokenize;
pub mod transformers;
pub mod utils;
mod validator;
pub mod vault;
use std::borrow::Cow;
#[cfg(feature = "v1")]
use std::collections::HashSet;
#[cfg(feature = "v2")]
use std::str::FromStr;

#[cfg(feature = "v2")]
pub use api_models::enums as api_enums;
pub use api_models::enums::Connector;
use api_models::payment_methods;
#[cfg(feature = "payouts")]
pub use api_models::{enums::PayoutConnectors, payouts as payout_types};
#[cfg(feature = "v1")]
use common_utils::{consts::DEFAULT_LOCALE, ext_traits::OptionExt};
#[cfg(feature = "v2")]
use common_utils::{
    crypto::{EncodeMessage, Encryptable, GcmAes256},
    encryption::Encryption,
    errors::CustomResult,
    ext_traits::{AsyncExt, BytesExt, ValueExt},
    fp_utils::when,
    generate_id, types as util_types,
};
use common_utils::{ext_traits::Encode, id_type};
use diesel_models::{
    enums, GenericLinkNew, PaymentMethodCollectLink, PaymentMethodCollectLinkData,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "v2")]
use futures::TryStreamExt;
#[cfg(feature = "v1")]
use hyperswitch_domain_models::api::{GenericLinks, GenericLinksData};
#[cfg(feature = "v2")]
use hyperswitch_domain_models::behaviour::Conversion;
#[cfg(feature = "v2")]
use hyperswitch_domain_models::payment_methods::PaymentMethodUpdate as DomainPaymentMethodUpdate;
use hyperswitch_domain_models::{
    payment_method_data::BankDebitData,
    payments::{payment_attempt::PaymentAttempt, PaymentIntent, VaultData},
    router_data_v2::flow_common_types::VaultConnectorFlowData,
    router_flow_types::ExternalVaultInsertFlow,
    types::VaultRouterData,
};
use hyperswitch_interfaces::connector_integration_interface::RouterDataConversion;
#[cfg(feature = "v2")]
use masking::ExposeInterface;
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};
use time::Duration;

#[cfg(feature = "v2")]
use super::payments::tokenization;
use super::{
    errors::{RouterResponse, StorageErrorExt},
    pm_auth,
};
#[cfg(feature = "v2")]
use crate::{
    configs::settings,
    core::{payment_methods::transformers as pm_transforms, tokenization as tokenization_core},
    headers,
    routes::{self, payment_methods as pm_routes},
    services::encryption,
    types::{
        api::PaymentMethodCreateExt,
        domain::types as domain_types,
        storage::{ephemeral_key, PaymentMethodListContext},
        transformers::{ForeignFrom, ForeignTryFrom},
        Tokenizable,
    },
    utils::ext_traits::OptionExt,
};
use crate::{
    consts,
    core::{
        errors::{ProcessTrackerError, RouterResult},
        payments::{self as payments_core, helpers as payment_helpers},
        utils as core_utils,
    },
    db::errors::ConnectorErrorExt,
    errors, logger,
    routes::{app::StorageInterface, SessionState},
    services,
    types::{
        self, api, domain, payment_methods as pm_types,
        storage::{self, enums as storage_enums},
    },
};

const PAYMENT_METHOD_STATUS_UPDATE_TASK: &str = "PAYMENT_METHOD_STATUS_UPDATE";
const PAYMENT_METHOD_STATUS_TAG: &str = "PAYMENT_METHOD_STATUS";
#[cfg(feature = "v2")]
const PAYMENT_METHOD_REDACTED_FINGERPRINT_ID: &str = "FINGERPRINT_ID_REDACTED";

#[instrument(skip_all)]
pub async fn retrieve_payment_method_core(
    pm_data: &Option<domain::PaymentMethodData>,
    state: &SessionState,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    provider: &domain::Provider,
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
                provider.get_key_store(),
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
                provider.get_key_store(),
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
                provider.get_key_store(),
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
                provider.get_key_store(),
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
                provider.get_key_store(),
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
    platform: domain::Platform,
    req: payment_methods::PaymentMethodCollectLinkRequest,
) -> RouterResponse<payment_methods::PaymentMethodCollectLinkResponse> {
    // Validate request and initiate flow
    let pm_collect_link_data =
        validator::validate_request_and_initiate_payment_method_collect_link(
            &state, &platform, &req,
        )
        .await?;

    // Create DB entry
    let pm_collect_link = create_pm_collect_db_entry(
        &state,
        &platform,
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
                format!("Failed to parse the payment method collect link - {url}")
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
    platform: &domain::Platform,
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
        merchant_id: platform.get_processor().get_account().get_id().to_owned(),
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

#[cfg(feature = "v2")]
pub async fn render_pm_collect_link(
    _state: SessionState,
    _platform: domain::Platform,
    _req: payment_methods::PaymentMethodCollectLinkRenderRequest,
) -> RouterResponse<services::GenericLinkFormData> {
    todo!()
}

#[cfg(feature = "v1")]
pub async fn render_pm_collect_link(
    state: SessionState,
    provider: domain::Provider,
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
                        &customer_id,
                        &req.merchant_id,
                        provider.get_key_store(),
                        provider.get_account().storage_scheme,
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
                    publishable_key: Secret::new(provider.get_account().clone().publishable_key),
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
                    sdk_url: default_config.sdk_url.clone(),
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

#[cfg(feature = "v1")]
pub async fn add_payment_method_status_update_task(
    db: &dyn StorageInterface,
    payment_method: &domain::PaymentMethod,
    prev_status: enums::PaymentMethodStatus,
    curr_status: enums::PaymentMethodStatus,
    merchant_id: &id_type::MerchantId,
    application_source: common_enums::ApplicationSource,
    initiator: Option<&hyperswitch_domain_models::platform::Initiator>,
) -> Result<(), ProcessTrackerError> {
    let created_at = payment_method.created_at;
    let schedule_time =
        created_at.saturating_add(Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

    let tracking_data = storage::PaymentMethodStatusTrackingData {
        payment_method_id: payment_method.get_id().clone(),
        prev_status,
        curr_status,
        merchant_id: merchant_id.to_owned(),
        last_modified_by: initiator
            .and_then(|initiator| initiator.to_created_by())
            .map(|last_modified_by| last_modified_by.to_string()),
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
        None,
        schedule_time,
        common_types::consts::API_VERSION,
        application_source,
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

#[cfg(feature = "v1")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn retrieve_payment_method_with_token(
    state: &SessionState,
    platform: &domain::Platform,
    token_data: &storage::PaymentTokenData,
    payment_intent: &PaymentIntent,
    payment_attempt: &PaymentAttempt,
    card_token_data: Option<&domain::CardToken>,
    storage_scheme: common_enums::enums::MerchantStorageScheme,
    mandate_id: Option<api_models::payments::MandateIds>,
    payment_method_info: Option<domain::PaymentMethod>,
    business_profile: &domain::Profile,
    should_retry_with_pan: bool,
    vault_data: Option<&VaultData>,
) -> RouterResult<storage::PaymentMethodDataWithId> {
    let token = match token_data {
        storage::PaymentTokenData::TemporaryGeneric(generic_token) => {
            payment_helpers::retrieve_payment_method_with_temporary_token(
                state,
                &generic_token.token,
                payment_intent,
                payment_attempt,
                platform.get_provider().get_key_store(),
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
                platform.get_provider().get_key_store(),
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

        storage::PaymentTokenData::Permanent(card_token) => Box::pin(
            payment_helpers::retrieve_payment_method_data_with_permanent_token(
                state,
                card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                card_token
                    .payment_method_id
                    .as_ref()
                    .unwrap_or(&card_token.token),
                payment_intent,
                card_token_data,
                platform,
                storage_scheme,
                mandate_id,
                payment_method_info
                    .get_required_value("PaymentMethod")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("PaymentMethod not found")?,
                business_profile,
                payment_attempt.connector.clone(),
                should_retry_with_pan,
                vault_data,
            ),
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
        .unwrap_or_default(),

        storage::PaymentTokenData::PermanentCard(card_token) => Box::pin(
            payment_helpers::retrieve_payment_method_data_with_permanent_token(
                state,
                card_token.locker_id.as_ref().unwrap_or(&card_token.token),
                card_token
                    .payment_method_id
                    .as_ref()
                    .unwrap_or(&card_token.token),
                payment_intent,
                card_token_data,
                platform,
                storage_scheme,
                mandate_id,
                payment_method_info
                    .get_required_value("PaymentMethod")
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("PaymentMethod not found")?,
                business_profile,
                payment_attempt.connector.clone(),
                should_retry_with_pan,
                vault_data,
            ),
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
        .unwrap_or_default(),

        storage::PaymentTokenData::AuthBankDebit(auth_token) => {
            pm_auth::retrieve_payment_method_from_auth_service(
                state,
                platform.get_processor(),
                auth_token,
                payment_intent,
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
        storage::PaymentTokenData::BankDebit(bank_debit) => {
            let customer_id = payment_intent.customer_id.as_ref().ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "customer",
                },
            )?;

            let locker_id = bank_debit.locker_id.as_ref().ok_or(
                errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "locker_id",
                },
            )?;

            let bank_debit_detail = cards::get_bank_debit_from_hs_locker(
                state,
                platform.get_provider(),
                customer_id,
                locker_id,
            )
            .await?;

            let (
                account_number,
                routing_number,
                vault_bank_account_holder_name,
                vault_bank_type,
                vault_bank_holder_type,
            ) = match bank_debit_detail {
                payment_methods::BankDebitDetail::Ach {
                    account_number,
                    routing_number,
                    bank_account_holder_name,
                    bank_type,
                    bank_holder_type,
                } => (
                    account_number,
                    routing_number,
                    bank_account_holder_name,
                    bank_type,
                    bank_holder_type,
                ),
            };

            let payment_method_data = payment_method_info
                .get_required_value("PaymentMethod")
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("PaymentMethod not found")?;

            let (
                card_holder_name,
                bank_account_holder_name,
                bank_name,
                bank_type,
                bank_holder_type,
            ) = if let Some(
                hyperswitch_domain_models::payment_method_data::PaymentMethodsData::BankDebit(
                    bank_debit_data,
                ),
            ) = payment_method_data.get_payment_methods_data()
            {
                use hyperswitch_domain_models::payment_method_data::BankDebitDetailsPaymentMethod;

                let BankDebitDetailsPaymentMethod::AchBankDebit {
                    masked_account_number: _,
                    masked_routing_number: _,
                    card_holder_name,
                    bank_account_holder_name,
                    bank_name,
                    bank_type,
                    bank_holder_type,
                } = bank_debit_data;
                (
                    card_holder_name.clone(),
                    bank_account_holder_name.clone(),
                    bank_name,
                    bank_type,
                    bank_holder_type,
                )
            } else {
                return Err(report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Payment method data is not bank debit"));
            };

            storage::PaymentMethodDataWithId {
                payment_method: Some(enums::PaymentMethod::BankDebit),
                payment_method_data: Some(
                    hyperswitch_domain_models::payment_method_data::PaymentMethodData::BankDebit(
                        BankDebitData::AchBankDebit {
                            account_number,
                            routing_number,
                            card_holder_name,
                            bank_account_holder_name: vault_bank_account_holder_name
                                .or(bank_account_holder_name),
                            bank_name,
                            bank_type: vault_bank_type.or(bank_type),
                            bank_holder_type: vault_bank_holder_type.or(bank_holder_type),
                        },
                    ),
                ),

                payment_method_id: Some(bank_debit.payment_method_id.clone()),
            }
        }
    };
    Ok(token)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub(crate) fn get_payment_method_create_request(
    payment_method_data: &api_models::payments::PaymentMethodData,
    payment_method_type: storage_enums::PaymentMethod,
    payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    customer_id: Option<id_type::GlobalCustomerId>,
    billing_address: Option<&api_models::payments::Address>,
    payment_method_session: Option<&domain::payment_methods::PaymentMethodSession>,
    storage_type: common_enums::StorageType,
) -> RouterResult<payment_methods::PaymentMethodCreate> {
    match payment_method_data {
        api_models::payments::PaymentMethodData::Card(card) => {
            payments_core::helpers::validate_card_expiry(
                &card.card_exp_month,
                &card.card_exp_year,
            )?;

            let card_detail = payment_methods::CardDetail {
                card_number: card.card_number.clone(),
                card_exp_month: card.card_exp_month.clone(),
                card_exp_year: card.card_exp_year.clone(),
                card_holder_name: card.card_holder_name.clone(),
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
                card_cvc: Some(card.card_cvc.clone()),
            };
            let payment_method_request = payment_methods::PaymentMethodCreate {
                payment_method_type,
                payment_method_subtype,
                metadata: None,
                customer_id,
                payment_method_data: payment_methods::PaymentMethodCreateData::Card(card_detail),
                billing: billing_address.map(ToOwned::to_owned),
                psp_tokenization: payment_method_session
                    .and_then(|pm_session| pm_session.psp_tokenization.clone()),
                network_tokenization: payment_method_session
                    .and_then(|pm_session| pm_session.network_tokenization.clone()),
                storage_type,
            };
            Ok(payment_method_request)
        }
        _ => Err(report!(errors::ApiErrorResponse::UnprocessableEntity {
            message: "only card payment methods are supported for tokenization".to_string()
        })
        .attach_printable("Payment method data is incorrect")),
    }
}

#[cfg(feature = "v1")]
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
                    let card_network = get_card_network_with_us_local_debit_network_override(
                        card.card_network.clone(),
                        card.co_badged_card_data.as_ref(),
                    );

                    let card_detail = payment_methods::CardDetail {
                        card_number: card.card_number.clone(),
                        card_exp_month: card.card_exp_month.clone(),
                        card_exp_year: card.card_exp_year.clone(),
                        card_holder_name: billing_name,
                        nick_name: card.nick_name.clone(),
                        card_issuing_country: card.card_issuing_country.clone(),
                        card_issuing_country_code: card.card_issuing_country_code.clone(),
                        card_network: card_network.clone(),
                        card_issuer: card.card_issuer.clone(),
                        card_type: card.card_type.clone(),
                        card_cvc: None, // DO NOT POPULATE CVC FOR ADDITIONAL PAYMENT METHOD DATA
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
                        card_network: card_network
                            .clone()
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
                domain::PaymentMethodData::BankDebit(BankDebitData::AchBankDebit {
                    account_number,
                    routing_number,
                    card_holder_name: _,
                    bank_account_holder_name,
                    bank_name: _,
                    bank_type,
                    bank_holder_type,
                }) => {
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
                        payment_method_data: Some(
                            payment_methods::PaymentMethodCreateData::BankDebit(
                                payment_methods::BankDebitDetail::Ach {
                                    account_number: account_number.to_owned(),
                                    routing_number: routing_number.to_owned(),
                                    bank_account_holder_name: bank_account_holder_name.clone(),
                                    bank_type: *bank_type,
                                    bank_holder_type: *bank_holder_type,
                                },
                            ),
                        ),
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

/// Determines the appropriate card network to to be stored.
///
/// If the provided card network is a US local network, this function attempts to
/// override it with the first global network from the co-badged card data, if available.
/// Otherwise, it returns the original card network as-is.
///
fn get_card_network_with_us_local_debit_network_override(
    card_network: Option<common_enums::CardNetwork>,
    co_badged_card_data: Option<&payment_methods::CoBadgedCardData>,
) -> Option<common_enums::CardNetwork> {
    if let Some(true) = card_network
        .as_ref()
        .map(|network| network.is_us_local_network())
    {
        services::logger::debug!("Card network is a US local network, checking for global network in co-badged card data");
        let info: Option<api_models::open_router::CoBadgedCardNetworksInfo> = co_badged_card_data
            .and_then(|data| {
                data.co_badged_card_networks_info
                    .0
                    .iter()
                    .find(|info| info.network.is_signature_network())
                    .cloned()
            });
        info.map(|data| data.network)
    } else {
        card_network
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn get_card_nt_eligibility(
    state: &SessionState,
    req: api::NetworkTokenEligibilityRequest,
    _platform: &domain::Platform,
    profile: &domain::Profile,
) -> RouterResponse<api::GetNetworkTokenEiligibilityResponse> {
    when(!profile.is_network_tokenization_enabled, || {
        Err(report!(errors::ApiErrorResponse::NotSupported {
            message: "Network tokenization is not enabled for this profile".to_string()
        }))
    })?;

    let response = network_tokenization::make_nt_eligibility_call(state, req)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to fetch network token eligibility from tokenization service")?;

    Ok(services::ApplicationResponse::Json(
        api::GetNetworkTokenEiligibilityResponse {
            eligible_for_network_tokenization: response.tokenize_support,
        },
    ))
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn create_payment_method(
    state: &SessionState,
    request_state: &routes::app::ReqState,
    req: api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
) -> RouterResponse<api::PaymentMethodResponse> {
    // payment_method is for internal use, can never be populated in response
    let (response, _payment_method) = Box::pin(create_payment_method_core(
        state,
        request_state,
        req,
        platform,
        profile,
    ))
    .await?;

    Ok(services::ApplicationResponse::Json(response))
}
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn create_payment_method_core(
    state: &SessionState,
    _request_state: &routes::app::ReqState,
    req: api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    match req.storage_type {
        common_enums::StorageType::Volatile => {
            create_volatile_payment_method_core(state, _request_state, req, platform, profile).await
        }
        common_enums::StorageType::Persistent => {
            create_persistent_payment_method_core(state, _request_state, req, platform, profile)
                .await
        }
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn create_persistent_payment_method_core(
    state: &SessionState,
    _request_state: &routes::app::ReqState,
    req: api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    req.validate()?;

    let db = &*state.store;
    let merchant_id = platform.get_provider().get_account().get_id();
    let customer_id = req
        .customer_id
        .to_owned()
        .get_required_value("customer_id")?;
    let key_manager_state = &(state).into();

    db.find_customer_by_global_id_merchant_id(
        &customer_id,
        platform.get_provider().get_account().get_id(),
        platform.get_provider().get_key_store(),
        platform.get_provider().get_account().storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
    .attach_printable("Customer not found for the payment method")?;

    let payment_method_billing_address = req
        .billing
        .clone()
        .async_map(|billing| {
            cards::create_encrypted_data(
                key_manager_state,
                platform.get_provider().get_key_store(),
                billing,
            )
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?
        .map(|encoded_address| {
            encoded_address.deserialize_inner_value(|value| value.parse_value("address"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse Payment method billing address")?;

    let payment_method_id =
        id_type::GlobalPaymentMethodId::generate(&state.conf.cell_information.id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    match &req.payment_method_data {
        api::PaymentMethodCreateData::Card(_) => {
            Box::pin(create_or_fetch_payment_method_core(
                state,
                req,
                platform,
                profile,
                merchant_id,
                &customer_id,
                payment_method_id,
                payment_method_billing_address,
            ))
            .await
        }
        api::PaymentMethodCreateData::ProxyCard(_) => {
            create_payment_method_proxy_card_core(
                state,
                req,
                platform,
                profile,
                merchant_id,
                &customer_id,
                payment_method_id,
                payment_method_billing_address,
            )
            .await
        }
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn create_volatile_payment_method_core(
    state: &SessionState,
    _request_state: &routes::app::ReqState,
    req: api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    req.validate()?;

    let db = &*state.store;
    let merchant_id = platform.get_provider().get_account().get_id();
    let customer_id = req.customer_id.to_owned();
    let key_manager_state = &(state).into();

    if let Some(ref customer_id) = customer_id {
        db.find_customer_by_global_id_merchant_id(
            customer_id,
            platform.get_provider().get_account().get_id(),
            platform.get_provider().get_key_store(),
            platform.get_provider().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Customer not found for the payment method")?;
    }

    let payment_method_billing_address = req
        .billing
        .clone()
        .async_map(|billing| {
            cards::create_encrypted_data(
                key_manager_state,
                platform.get_provider().get_key_store(),
                billing,
            )
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method billing address")?
        .map(|encoded_address| {
            encoded_address.deserialize_inner_value(|value| value.parse_value("address"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse Payment method billing address")?;

    let payment_method_id =
        id_type::GlobalPaymentMethodId::generate(&state.conf.cell_information.id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to generate GlobalPaymentMethodId")?;

    match &req.payment_method_data {
        api::PaymentMethodCreateData::Card(_) => {
            logger::info!("Creating volatile card payment method");
            Box::pin(create_volatile_payment_method_card_core(
                state,
                req,
                platform,
                profile,
                merchant_id,
                &customer_id,
                payment_method_id,
                payment_method_billing_address,
            ))
            .await
        }
        api::PaymentMethodCreateData::ProxyCard(_) => {
            Err(report!(errors::ApiErrorResponse::UnprocessableEntity {
                message: "Proxy card payment method cannot be created as volatile".to_string()
            }))
        }
    }
}

#[cfg(feature = "v2")]
async fn payment_method_resolver(
    state: &SessionState,
    platform: &domain::Platform,
    customer_id: &id_type::GlobalCustomerId,
    req: &api::PaymentMethodCreate,
    fingerprint_id: String,
    payment_method_data: domain::PaymentMethodVaultingData,
) -> RouterResult<PaymentMethodResolver> {
    let db = &*state.store;

    match db
        .find_payment_method_by_fingerprint_id(
            platform.get_provider().get_key_store(),
            &fingerprint_id,
        )
        .await
    {
        Ok(existing_pm) => match existing_pm.status {
            enums::PaymentMethodStatus::New | enums::PaymentMethodStatus::Inactive => {
                logger::info!(
                        "Payment method is duplicated with update-eligible status, updating existing payment method"
                    );
                let payment_method_id = existing_pm.id.clone();
                Ok(PaymentMethodResolver(PaymentMethodResolution::Update {
                    fingerprint_id,
                    payment_method_id,
                    payment_method: Box::new(existing_pm),
                    source_payment_method_data: payment_method_data,
                }))
            }
            enums::PaymentMethodStatus::Active => {
                logger::info!("Payment method is duplicated, returning existing payment method");
                Ok(PaymentMethodResolver(PaymentMethodResolution::Get(
                    Box::new(existing_pm),
                )))
            }
            enums::PaymentMethodStatus::AwaitingData | enums::PaymentMethodStatus::Processing => {
                // no payment method entry will be found with finger print id and awaiting data or processing state
                logger::info!("Payment method is in awaiting data or processing state, no existing payment method entry found with fingerprint id");
                Err(
                    report!(errors::ApiErrorResponse::InternalServerError).attach_printable(
                        "Payment method is in invalid state, expected New or Inactive",
                    ),
                )
            }
        },
        Err(err) => {
            when(!err.current_context().is_db_not_found(), || {
                Err(err)
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to find payment method by fingerprint id")
            })?;

            logger::debug!("Payment method not found, falling back to creation");
            Ok(PaymentMethodResolver(PaymentMethodResolution::Create {
                fingerprint_id,
                payment_method_data,
            }))
        }
    }
}

#[cfg(feature = "v2")]
pub enum PaymentMethodResolution {
    Get(Box<domain::PaymentMethod>),
    Update {
        fingerprint_id: String,
        payment_method_id: id_type::GlobalPaymentMethodId,
        payment_method: Box<domain::PaymentMethod>,
        source_payment_method_data: domain::PaymentMethodVaultingData,
    },
    Create {
        fingerprint_id: String,
        payment_method_data: domain::PaymentMethodVaultingData,
    },
}

#[cfg(feature = "v2")]
pub struct PaymentMethodResolver(PaymentMethodResolution);

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn create_or_fetch_payment_method_core(
    state: &SessionState,
    req: api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
    merchant_id: &id_type::MerchantId,
    customer_id: &id_type::GlobalCustomerId,
    payment_method_id: id_type::GlobalPaymentMethodId,
    billing_address: Option<Encryptable<hyperswitch_domain_models::address::Address>>,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    let bin_enriched_payment_method_data =
        domain::PaymentMethodVaultingData::try_from(req.payment_method_data.clone())?
            .populate_bin_details_for_payment_method(state)
            .await;

    let payment_method_subtype = bin_enriched_payment_method_data
        .payment_method_subtype
        .or(req.payment_method_subtype);

    let payment_method_data = bin_enriched_payment_method_data.data;

    let fingerprint_id = vault::get_fingerprint_id_for_payment_method(
        state,
        &payment_method_data,
        customer_id.get_string_repr().to_owned(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get fingerprint_id from vault")?;

    let resolver = payment_method_resolver(
        state,
        platform,
        customer_id,
        &req,
        fingerprint_id,
        payment_method_data,
    )
    .await?;

    Box::pin(resolver.execute(
        state,
        &req,
        platform,
        profile,
        merchant_id,
        customer_id,
        payment_method_id,
        payment_method_subtype,
        billing_address,
    ))
    .await
}

#[cfg(feature = "v2")]
impl PaymentMethodResolver {
    #[allow(clippy::too_many_arguments)]
    #[instrument(skip_all)]
    pub async fn execute(
        self,
        state: &SessionState,
        req: &api::PaymentMethodCreate,
        platform: &domain::Platform,
        profile: &domain::Profile,
        merchant_id: &id_type::MerchantId,
        customer_id: &id_type::GlobalCustomerId,
        payment_method_id: id_type::GlobalPaymentMethodId,
        payment_method_subtype: Option<storage_enums::PaymentMethodType>,
        billing_address: Option<Encryptable<hyperswitch_domain_models::address::Address>>,
    ) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
        let db = &*state.store;
        match self.0 {
            PaymentMethodResolution::Get(existing_pm) => {
                logger::debug!("Payment method is duplicate, found {:?}", existing_pm.id);
                let card_cvc = req
                    .payment_method_data
                    .get_card()
                    .and_then(|card| card.card_cvc.clone());

                let cvc_expiry_details = if let Some(cvc) = card_cvc {
                    logger::debug!("Inserting CVC for payment method {:?}", existing_pm.id);
                    let intent_fulfillment_time =
                        common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME;
                    Some(
                        vault::insert_cvc_using_payment_token(
                            state,
                            &existing_pm.id,
                            cvc,
                            intent_fulfillment_time,
                            platform.get_provider().get_key_store(),
                        )
                        .await?,
                    )
                } else {
                    logger::debug!(
                        "No CVC found in the payment method request, trying to retrieve from redis"
                    );
                    let existing_cvc_expiry_details =
                        vault::retrieve_key_and_ttl_for_cvc_from_payment_method_id(
                            state,
                            existing_pm.id.to_owned(),
                        )
                        .await
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Failed to retrieve cvc from redis")
                        .ok()
                        .map(|time| {
                            payment_methods::CardCVCTokenStorageDetails::generate_expiry_timestamp(
                                time,
                            )
                        });
                    existing_cvc_expiry_details
                };
                let billing = billing_address
                    .clone()
                    .map(|billing| billing.into_inner())
                    .map(From::from);

                let resp = pm_transforms::generate_payment_method_response(
                    &existing_pm,
                    &None,
                    req.storage_type,
                    cvc_expiry_details,
                    req.customer_id.clone(),
                    None,
                    billing,
                    None,
                )?;

                Ok((resp, *existing_pm))
            }

            PaymentMethodResolution::Update {
                fingerprint_id,
                payment_method_id,
                payment_method,
                source_payment_method_data,
            } => {
                logger::debug!(
                    "Updating duplicated payment method {:?} for fingerprint {}",
                    payment_method_id,
                    fingerprint_id
                );

                let payment_method = *payment_method;
                let payment_method_has_network_token_data = payment_method
                    .network_token_requestor_reference_id
                    .is_some()
                    || payment_method.network_token_locker_id.is_some()
                    || payment_method.network_token_payment_method_data.is_some();
                let network_tokenization_resp = if req.network_tokenization.is_some()
                    && !payment_method_has_network_token_data
                {
                    let customer_id = payment_method
                        .customer_id
                        .clone()
                        .get_required_value("GlobalCustomerId")?;

                    network_tokenize_and_vault_the_pmd(
                        state,
                        &source_payment_method_data,
                        platform,
                        req.network_tokenization.clone(),
                        profile.is_network_tokenization_enabled,
                        &customer_id,
                    )
                    .await
                } else {
                    None
                };

                let update_payment_method_data: DomainPaymentMethodUpdate =
                    (req, source_payment_method_data).into();

                let (response, updated_payment_method) = Box::pin(update_payment_method_core(
                    state,
                    platform,
                    profile,
                    update_payment_method_data,
                    &payment_method_id,
                    Some(payment_method),
                    network_tokenization_resp,
                ))
                .await?;

                Ok((response, updated_payment_method))
            }

            PaymentMethodResolution::Create {
                fingerprint_id,
                payment_method_data,
            } => {
                let payment_method = create_payment_method_for_intent(
                    state,
                    req.metadata.clone(),
                    customer_id,
                    payment_method_id,
                    merchant_id,
                    platform.get_provider().get_key_store(),
                    platform.get_provider().get_account().storage_scheme,
                    billing_address.clone(),
                    platform.get_initiator(),
                )
                .await?;
                Box::pin(execute_payment_method_create(
                    state,
                    req,
                    platform,
                    profile,
                    customer_id,
                    payment_method,
                    payment_method_subtype,
                    payment_method_data,
                    fingerprint_id,
                    billing_address,
                ))
                .await
            }
        }
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn execute_payment_method_create(
    state: &SessionState,
    req: &api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
    customer_id: &id_type::GlobalCustomerId,
    payment_method: domain::PaymentMethod,
    payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    payment_method_data: domain::PaymentMethodVaultingData,
    fingerprint_id_from_vault: String,
    payment_method_billing_address: Option<
        Encryptable<hyperswitch_domain_models::address::Address>,
    >,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    match &req.payment_method_data {
        api::PaymentMethodCreateData::Card(card_data) => {
            payments_core::helpers::validate_card_expiry(
                &card_data.card_exp_month,
                &card_data.card_exp_year,
            )?;
        }
        _ => {
            logger::debug!("Payment method data is not CardDetail");
        }
    }
    let db = &*state.store;

    let vaulting_result = vault_payment_method(
        state,
        &payment_method_data,
        platform,
        profile,
        None,
        fingerprint_id_from_vault,
        customer_id,
    )
    .await;

    let network_tokenization_resp = network_tokenize_and_vault_the_pmd(
        state,
        &payment_method_data,
        platform,
        req.network_tokenization.clone(),
        profile.is_network_tokenization_enabled,
        customer_id,
    )
    .await;

    let (response, payment_method) = match vaulting_result {
        Ok((
            pm_types::AddVaultResponse {
                vault_id,
                fingerprint_id,
                ..
            },
            external_vault_source,
        )) => {
            let pm_update = create_pm_additional_data_update(
                Some(&payment_method_data),
                state,
                platform.get_provider().get_key_store(),
                Some(vault_id.get_string_repr().clone()),
                fingerprint_id,
                &payment_method,
                None,
                None,
                network_tokenization_resp,
                Some(req.payment_method_type),
                payment_method_subtype,
                external_vault_source,
                Some(enums::PaymentMethodStatus::New),
                platform.get_initiator(),
            )
            .await
            .attach_printable("unable to create payment method data")?;

            let payment_method = db
                .update_payment_method(
                    platform.get_provider().get_key_store(),
                    payment_method,
                    pm_update,
                    platform.get_provider().get_account().storage_scheme,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to update payment method in db")?;

            let intent_fulfillment_time = common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME;

            let card_cvc = req
                .payment_method_data
                .get_card()
                .and_then(|card| card.card_cvc.clone());

            let cvc_expiry_details = card_cvc
                .async_map(|cvc| {
                    vault::insert_cvc_using_payment_token(
                        state,
                        &payment_method.id,
                        cvc,
                        intent_fulfillment_time,
                        platform.get_provider().get_key_store(),
                    )
                })
                .await
                .transpose()?;

            let resp = pm_transforms::generate_payment_method_response(
                &payment_method,
                &None,
                req.storage_type,
                cvc_expiry_details,
                req.customer_id.clone(),
                None,
                payment_method_billing_address.map(|add| add.get_inner().clone().into()),
                None,
            )?;

            Ok((resp, payment_method))
        }
        Err(e) => {
            let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
                status: Some(enums::PaymentMethodStatus::Inactive),
                last_modified_by: platform
                    .get_initiator()
                    .and_then(|initiator| initiator.to_created_by())
                    .map(|last_modified_by| last_modified_by.to_string()),
            };

            db.update_payment_method(
                platform.get_provider().get_key_store(),
                payment_method,
                pm_update,
                platform.get_provider().get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;

            Err(e)
        }
    }?;

    Ok((response, payment_method))
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn create_volatile_payment_method_card_core(
    state: &SessionState,
    req: api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
    merchant_id: &id_type::MerchantId,
    customer_id: &Option<id_type::GlobalCustomerId>,
    payment_method_id: id_type::GlobalPaymentMethodId,
    payment_method_billing_address: Option<
        Encryptable<hyperswitch_domain_models::address::Address>,
    >,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    let db = &*state.store;
    let keymanager_state = &(state).into();
    let merchant_key_store = platform.get_provider().get_key_store();

    let bin_enriched_payment_method_data =
        domain::PaymentMethodVaultingData::try_from(req.payment_method_data.clone())?
            .populate_bin_details_for_payment_method(state)
            .await;
    let payment_method_subtype = bin_enriched_payment_method_data
        .payment_method_subtype
        .or(req.payment_method_subtype);
    let payment_method_data = bin_enriched_payment_method_data.data;

    let vaulting_result = vault_payment_method_in_volatile_storage(
        state,
        &payment_method_data,
        platform,
        profile,
        None,
        customer_id,
    )
    .await;

    let (response, payment_method) = match vaulting_result {
        Ok((
            pm_types::AddVaultResponse {
                vault_id,
                fingerprint_id,
                ..
            },
            external_vault_source,
        )) => {
            let locker_id = Some(vault_id.clone());
            let payment_method = construct_payment_method_object(
                req.metadata.clone(),
                customer_id,
                payment_method_id,
                merchant_id,
                payment_method_billing_address.clone(),
                state,
                platform.get_provider().get_key_store(),
                Some(&payment_method_data),
                Some(req.payment_method_type),
                payment_method_subtype,
                locker_id,
                fingerprint_id,
                external_vault_source,
                platform.get_initiator(),
            )
            .await
            .attach_printable("failed to construct payment method")?
            .convert()
            .await
            .change_context(errors::StorageError::EncryptionError)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to convert payment method")?; //Convert to storage model payment method to store in redis

            let redis_connection = state
                .store
                .get_redis_conn()
                .map_err(Into::<errors::StorageError>::into)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to get redis connection")?;

            logger::info!("Storing payment method id in redis");

            redis_connection
                .serialize_and_set_key_with_expiry(
                    &payment_method.get_id().get_string_repr().to_string().into(),
                    payment_method.clone(),
                    consts::DEFAULT_PAYMENT_METHOD_STORE_TTL,
                )
                .await
                .map_err(Into::<errors::StorageError>::into)
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to insert payment method id in redis")?;

            let domain_payment_method = domain::PaymentMethod::convert_back(
                keymanager_state,
                payment_method,
                merchant_key_store.key.get_inner(),
                merchant_key_store.merchant_id.clone().into(),
            )
            .await
            .change_context(errors::StorageError::DecryptionError)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to convert payment method")?;

            let intent_fulfillment_time = common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME;

            let card_cvc = req
                .payment_method_data
                .get_card()
                .and_then(|card| card.card_cvc.clone());

            let cvc_expiry_details = card_cvc
                .async_map(|cvc| {
                    vault::insert_cvc_using_payment_token(
                        state,
                        &domain_payment_method.id,
                        cvc,
                        intent_fulfillment_time,
                        platform.get_provider().get_key_store(),
                    )
                })
                .await
                .transpose()?;

            let resp = pm_transforms::generate_payment_method_response(
                &domain_payment_method,
                &None,
                req.storage_type,
                cvc_expiry_details,
                req.customer_id,
                None,
                None,
                None,
            )?;

            Ok((resp, domain_payment_method))
        }
        Err(e) => Err(e),
    }?;

    Ok((response, payment_method))
}

// network tokenization and vaulting to locker is not required for proxy card since the card is already tokenized
#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn create_payment_method_proxy_card_core(
    state: &SessionState,
    req: api::PaymentMethodCreate,
    platform: &domain::Platform,
    profile: &domain::Profile,
    merchant_id: &id_type::MerchantId,
    customer_id: &id_type::GlobalCustomerId,
    payment_method_id: id_type::GlobalPaymentMethodId,
    payment_method_billing_address: Option<
        Encryptable<hyperswitch_domain_models::address::Address>,
    >,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    use crate::core::payment_methods::vault::Vault;

    let key_manager_state = &(state).into();

    let external_vault_source = profile
        .external_vault_connector_details
        .clone()
        .map(|details| details.vault_connector_id);

    let bin_enriched_payment_method_data = req
        .payment_method_data
        .populate_bin_details_for_payment_method(state)
        .await;
    let payment_method_subtype = bin_enriched_payment_method_data
        .payment_method_subtype
        .or(req.payment_method_subtype);
    let additional_payment_method_data = Some(
        bin_enriched_payment_method_data
            .data
            .convert_to_additional_payment_method_data()?,
    );

    let encrypted_payment_method_data = additional_payment_method_data
        .async_map(|payment_method_data| {
            cards::create_encrypted_data(
                key_manager_state,
                platform.get_provider().get_key_store(),
                payment_method_data,
            )
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt Payment method data")?
        .map(|encoded_pmd| {
            encoded_pmd.deserialize_inner_value(|value| value.parse_value("PaymentMethodsData"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse Payment method data")?;

    let external_vault_token_data = req.payment_method_data.get_external_vault_token_data();

    let encrypted_external_vault_token_data = external_vault_token_data
        .async_map(|external_vault_token_data| {
            cards::create_encrypted_data(
                key_manager_state,
                platform.get_provider().get_key_store(),
                external_vault_token_data,
            )
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to encrypt External vault token data")?
        .map(|encoded_data| {
            encoded_data
                .deserialize_inner_value(|value| value.parse_value("ExternalVaultTokenData"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse External vault token data")?;

    let vault_type = external_vault_source
        .is_some()
        .then_some(common_enums::VaultType::External);

    let payment_method = create_payment_method_for_confirm(
        state,
        customer_id,
        payment_method_id,
        external_vault_source,
        merchant_id,
        platform.get_provider().get_key_store(),
        platform.get_provider().get_account().storage_scheme,
        req.payment_method_type,
        payment_method_subtype,
        payment_method_billing_address,
        encrypted_payment_method_data,
        encrypted_external_vault_token_data,
        vault_type,
        platform.get_initiator(),
    )
    .await?;

    let payment_method_response = pm_transforms::generate_payment_method_response(
        &payment_method,
        &None,
        req.storage_type,
        None,
        req.customer_id,
        None,
        None,
        None,
    )?;

    Ok((payment_method_response, payment_method))
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct NetworkTokenPaymentMethodDetails {
    network_token_requestor_reference_id: String,
    network_token_locker_id: String,
    network_token_pmd: Encryptable<Secret<serde_json::Value>>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug)]
pub struct BinEnriched<T> {
    pub data: T,
    pub payment_method_subtype: Option<storage_enums::PaymentMethodType>,
}

#[cfg(feature = "v2")]
pub async fn network_tokenize_and_vault_the_pmd(
    state: &SessionState,
    payment_method_data: &domain::PaymentMethodVaultingData,
    platform: &domain::Platform,
    network_tokenization: Option<common_types::payment_methods::NetworkTokenization>,
    network_tokenization_enabled_for_profile: bool,
    customer_id: &id_type::GlobalCustomerId,
) -> Option<NetworkTokenPaymentMethodDetails> {
    let network_token_pm_details_result: CustomResult<
        NetworkTokenPaymentMethodDetails,
        errors::NetworkTokenizationError,
    > = async {
        when(!network_tokenization_enabled_for_profile, || {
            Err(report!(
                errors::NetworkTokenizationError::NetworkTokenizationNotEnabledForProfile
            ))
        })?;

        let is_network_tokenization_enabled_for_pm = network_tokenization
            .as_ref()
            .map(|nt| matches!(nt.enable, common_enums::NetworkTokenizationToggle::Enable))
            .unwrap_or(false);

        let card_data = payment_method_data
            .get_card()
            .and_then(|card| is_network_tokenization_enabled_for_pm.then_some(card))
            .ok_or_else(|| {
                report!(errors::NetworkTokenizationError::NotSupported {
                    message: "Payment method".to_string(),
                })
            })?;

        let (resp, network_token_req_ref_id) =
            network_tokenization::make_card_network_tokenization_request(
                state,
                card_data,
                customer_id,
            )
            .await?;

        let network_token_vaulting_data = domain::PaymentMethodVaultingData::NetworkToken(resp);
        let vaulting_resp = vault::add_payment_method_to_vault(
            state,
            platform,
            &network_token_vaulting_data,
            None,
            customer_id,
        )
        .await
        .change_context(errors::NetworkTokenizationError::SaveNetworkTokenFailed)
        .attach_printable("Failed to vault network token")?;

        let key_manager_state = &(state).into();
        let network_token_pmd = cards::create_encrypted_data(
            key_manager_state,
            platform.get_provider().get_key_store(),
            network_token_vaulting_data.get_payment_methods_data(),
        )
        .await
        .change_context(errors::NetworkTokenizationError::NetworkTokenDetailsEncryptionFailed)
        .attach_printable("Failed to encrypt PaymentMethodsData")?;

        Ok(NetworkTokenPaymentMethodDetails {
            network_token_requestor_reference_id: network_token_req_ref_id,
            network_token_locker_id: vaulting_resp.vault_id.get_string_repr().clone(),
            network_token_pmd,
        })
    }
    .await;
    network_token_pm_details_result.ok()
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
pub trait PaymentMethodExt {
    async fn populate_bin_details_for_payment_method(
        &self,
        state: &SessionState,
    ) -> BinEnriched<Self>
    where
        Self: Sized;

    // convert to data format compatible to save in payment method table
    fn convert_to_additional_payment_method_data(
        &self,
    ) -> RouterResult<payment_methods::PaymentMethodsData>;

    // get tokens generated from external vault
    fn get_external_vault_token_data(&self) -> Option<payment_methods::ExternalVaultTokenData>;
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl PaymentMethodExt for domain::PaymentMethodVaultingData {
    async fn populate_bin_details_for_payment_method(
        &self,
        state: &SessionState,
    ) -> BinEnriched<Self> {
        match self {
            Self::Card(card) => {
                let card_isin = card.card_number.get_card_isin();

                let card_info = state
                    .store
                    .get_card_info(&card_isin)
                    .await
                    .map_err(|error| services::logger::error!(card_info_error=?error))
                    .ok()
                    .flatten();

                let additional_payment_data_from_bin_lookup = card_info.as_ref().map(|val| {
                    api_models::payments::AdditionalPaymentData::Card(Box::new(
                        api_models::payments::AdditionalCardInfo {
                            card_type: val.card_type.clone(),
                            ..Default::default()
                        },
                    ))
                });

                let payment_method_subtype =
                    Option::<storage_enums::PaymentMethodType>::foreign_from((
                        None,
                        additional_payment_data_from_bin_lookup.as_ref(),
                        Some(storage_enums::PaymentMethod::Card),
                    ));

                if card.card_issuer.is_some()
                    && card.card_network.is_some()
                    && card.card_type.is_some()
                    && card.card_issuing_country.is_some()
                {
                    BinEnriched {
                        data: Self::Card(card.clone()),
                        payment_method_subtype,
                    }
                } else {
                    BinEnriched {
                        data: Self::Card(payment_methods::CardDetail {
                            card_number: card.card_number.clone(),
                            card_exp_month: card.card_exp_month.clone(),
                            card_exp_year: card.card_exp_year.clone(),
                            card_holder_name: card.card_holder_name.clone(),
                            nick_name: card.nick_name.clone(),
                            card_issuing_country: card_info.as_ref().and_then(|val| {
                                val.card_issuing_country
                                    .as_ref()
                                    .map(|c| api_enums::CountryAlpha2::from_str(c))
                                    .transpose()
                                    .ok()
                                    .flatten()
                            }),
                            card_network: card_info
                                .as_ref()
                                .and_then(|val| val.card_network.clone()),
                            card_issuer: card_info.as_ref().and_then(|val| val.card_issuer.clone()),
                            card_type: card_info.as_ref().and_then(|val| {
                                val.card_type
                                    .as_ref()
                                    .map(|c| payment_methods::CardType::from_str(c))
                                    .transpose()
                                    .ok()
                                    .flatten()
                            }),
                            card_cvc: card.card_cvc.clone(),
                        }),
                        payment_method_subtype,
                    }
                }
            }
            _ => BinEnriched {
                data: self.clone(),
                payment_method_subtype: None,
            },
        }
    }

    // Not implement because it is saved in locker and not in payment method table
    fn convert_to_additional_payment_method_data(
        &self,
    ) -> RouterResult<payment_methods::PaymentMethodsData> {
        Err(report!(errors::ApiErrorResponse::UnprocessableEntity {
            message: "Payment method data is not supported".to_string()
        })
        .attach_printable("Payment method data is not supported"))
    }

    fn get_external_vault_token_data(&self) -> Option<payment_methods::ExternalVaultTokenData> {
        None
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl PaymentMethodExt for payment_methods::PaymentMethodCreateData {
    async fn populate_bin_details_for_payment_method(
        &self,
        state: &SessionState,
    ) -> BinEnriched<Self> {
        match self {
            Self::ProxyCard(card) => {
                let card_isin = card.bin_number.clone();
                let card_info = if let Some(card_isin) = card_isin.as_ref() {
                    state
                        .store
                        .get_card_info(card_isin)
                        .await
                        .map_err(|error| services::logger::error!(card_info_error=?error))
                        .ok()
                        .flatten()
                } else {
                    None
                };

                let additional_payment_data_from_bin_lookup = card_info.as_ref().map(|val| {
                    api_models::payments::AdditionalPaymentData::Card(Box::new(
                        api_models::payments::AdditionalCardInfo {
                            card_type: val.card_type.clone(),
                            ..Default::default()
                        },
                    ))
                });

                let payment_method_subtype =
                    Option::<storage_enums::PaymentMethodType>::foreign_from((
                        None,
                        additional_payment_data_from_bin_lookup.as_ref(),
                        Some(storage_enums::PaymentMethod::Card),
                    ));

                if card.card_issuer.is_some()
                    && card.card_network.is_some()
                    && card.card_type.is_some()
                    && card.card_issuing_country.is_some()
                {
                    BinEnriched {
                        data: Self::ProxyCard(card.clone()),
                        payment_method_subtype,
                    }
                } else if card_isin.is_some() {
                    BinEnriched {
                        data: Self::ProxyCard(payment_methods::ProxyCardDetails {
                            card_number: card.card_number.clone(),
                            card_exp_month: card.card_exp_month.clone(),
                            card_exp_year: card.card_exp_year.clone(),
                            card_holder_name: card.card_holder_name.clone(),
                            bin_number: card.bin_number.clone(),
                            last_four: card.last_four.clone(),
                            nick_name: card.nick_name.clone(),
                            card_issuing_country: card_info
                                .as_ref()
                                .and_then(|val| val.card_issuing_country.clone()),
                            card_network: card_info
                                .as_ref()
                                .and_then(|val| val.card_network.clone()),
                            card_issuer: card_info.as_ref().and_then(|val| val.card_issuer.clone()),
                            card_type: card_info.as_ref().and_then(|val| val.card_type.clone()),
                            card_cvc: card.card_cvc.clone(),
                        }),
                        payment_method_subtype,
                    }
                } else {
                    BinEnriched {
                        data: Self::ProxyCard(card.clone()),
                        payment_method_subtype,
                    }
                }
            }
            _ => BinEnriched {
                data: self.clone(),
                payment_method_subtype: None,
            },
        }
    }

    fn convert_to_additional_payment_method_data(
        &self,
    ) -> RouterResult<payment_methods::PaymentMethodsData> {
        match self.clone() {
            Self::ProxyCard(card_details) => Ok(payment_methods::PaymentMethodsData::Card(
                payment_methods::CardDetailsPaymentMethod {
                    last4_digits: card_details.last_four,
                    expiry_month: Some(card_details.card_exp_month),
                    expiry_year: Some(card_details.card_exp_year),
                    card_holder_name: card_details.card_holder_name,
                    nick_name: card_details.nick_name,
                    issuer_country: card_details.card_issuing_country,
                    issuer_country_code: None,
                    card_network: card_details.card_network,
                    card_issuer: card_details.card_issuer,
                    card_type: card_details.card_type,
                    card_isin: card_details.bin_number,
                    saved_to_locker: false,
                    co_badged_card_data: None,
                },
            )),
            Self::Card(card_details) => Ok(payment_methods::PaymentMethodsData::Card(
                payment_methods::CardDetailsPaymentMethod {
                    expiry_month: Some(card_details.card_exp_month),
                    expiry_year: Some(card_details.card_exp_year),
                    card_holder_name: card_details.card_holder_name,
                    nick_name: card_details.nick_name,
                    issuer_country: card_details
                        .card_issuing_country
                        .map(|country| country.to_string()),
                    card_network: card_details.card_network,
                    issuer_country_code: None,
                    card_issuer: card_details.card_issuer,
                    card_type: card_details
                        .card_type
                        .map(|card_type| card_type.to_string()),
                    saved_to_locker: false,
                    card_isin: None,
                    last4_digits: None,
                    co_badged_card_data: None,
                },
            )),
        }
    }

    fn get_external_vault_token_data(&self) -> Option<payment_methods::ExternalVaultTokenData> {
        match self.clone() {
            Self::ProxyCard(card_details) => Some(payment_methods::ExternalVaultTokenData {
                tokenized_card_number: card_details.card_number,
            }),
            Self::Card(_) => None,
        }
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn payment_method_intent_create(
    state: &SessionState,
    req: api::PaymentMethodIntentCreate,
    provider: domain::Provider,
    initiator: Option<&domain::Initiator>,
) -> RouterResponse<api::PaymentMethodResponse> {
    let db = &*state.store;
    let merchant_id = provider.get_account().get_id();
    let customer_id = req.customer_id.to_owned();
    let key_manager_state = &(state).into();

    db.find_customer_by_global_id_merchant_id(
        &customer_id,
        provider.get_account().get_id(),
        provider.get_key_store(),
        provider.get_account().storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
    .attach_printable("Customer not found for the payment method")?;

    let payment_method_billing_address = req
        .billing
        .clone()
        .async_map(|billing| {
            cards::create_encrypted_data(key_manager_state, provider.get_key_store(), billing)
        })
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
        provider.get_key_store(),
        provider.get_account().storage_scheme,
        payment_method_billing_address,
        initiator,
    )
    .await
    .attach_printable("Failed to add Payment method to DB")?;

    let resp = pm_transforms::generate_payment_method_response(
        &payment_method,
        &None,
        common_enums::StorageType::Persistent,
        None,
        Some(customer_id),
        None,
        None,
        None,
    )?;

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

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn list_payment_methods_for_session(
    state: SessionState,
    platform: domain::Platform,
    profile: domain::Profile,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
) -> RouterResponse<api::PaymentMethodListResponseForSession> {
    let db = &*state.store;

    let payment_method_session = db
        .get_payment_methods_session(
            platform.get_provider().get_key_store(),
            &payment_method_session_id,
        )
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    let payment_connector_accounts = db
        .list_enabled_connector_accounts_by_profile_id(
            profile.get_id(),
            platform.get_processor().get_key_store(),
            common_enums::ConnectorType::PaymentProcessor,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("error when fetching merchant connector accounts")?;

    let customer_payment_methods = match payment_method_session.customer_id {
        Some(ref customer_id) => {
            list_customer_payment_methods_core(
                &state,
                platform.get_provider(),
                customer_id,
                payment_method_session.keep_alive,
            )
            .await?
        }
        None => Vec::new(),
    };

    // Create associated payment methods from customer payment methods
    let associated_payment_methods_vec: Vec<common_types::payment_methods::AssociatedPaymentMethods> = customer_payment_methods
        .iter()
        .map(|cpm| {
            common_types::payment_methods::AssociatedPaymentMethods {
                payment_method_token: common_types::payment_methods::AssociatedPaymentMethodTokenType::PaymentMethodSessionToken(cpm.payment_method_token.clone()),
            }
        })
        .collect();

    // Update payment method session with associated payment methods
    let update_payment_method_session = hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum::UpdateAssociatedPaymentMethods {
        associated_payment_methods: Some(associated_payment_methods_vec.clone())
    };

    let updated_payment_method_session = db
        .update_payment_method_session(
            platform.get_provider().get_key_store(),
            &payment_method_session_id,
            update_payment_method_session,
            payment_method_session,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to update payment method session with associated payment methods",
        )?;

    let response =
        hyperswitch_domain_models::merchant_connector_account::FlattenedPaymentMethodsEnabled::from_payment_connectors_list(payment_connector_accounts)
            .perform_filtering()
            .get_required_fields(RequiredFieldsInput::new(state.conf.required_fields.clone()))
            .generate_response_for_session(customer_payment_methods);

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        response,
    ))
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[instrument(skip_all)]
pub async fn list_saved_payment_methods_for_customer(
    state: SessionState,
    provider: domain::Provider,
    customer_id: id_type::GlobalCustomerId,
    include_new: bool,
) -> RouterResponse<payment_methods::CustomerPaymentMethodsListResponse> {
    let customer_payment_methods =
        list_payment_methods_core(&state, &provider, &customer_id, include_new).await?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        customer_payment_methods,
    ))
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[instrument(skip_all)]
pub async fn get_token_data_for_payment_method(
    state: SessionState,
    provider: domain::Provider,
    profile: domain::Profile,
    request: payment_methods::GetTokenDataRequest,
    payment_method_id: id_type::GlobalPaymentMethodId,
) -> RouterResponse<api::TokenDataResponse> {
    let db = &*state.store;

    let payment_method = db
        .find_payment_method(
            provider.get_key_store(),
            &payment_method_id,
            provider.get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let token_data_response =
        generate_token_data_response(&state, request, profile, &payment_method).await?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        token_data_response,
    ))
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[instrument(skip_all)]
pub async fn generate_token_data_response(
    state: &SessionState,
    request: payment_methods::GetTokenDataRequest,
    profile: domain::Profile,
    payment_method: &domain::PaymentMethod,
) -> RouterResult<api::TokenDataResponse> {
    let token_details = match request.token_type {
        common_enums::TokenDataType::NetworkToken => {
            let is_network_tokenization_enabled = profile.is_network_tokenization_enabled;
            if !is_network_tokenization_enabled {
                return Err(errors::ApiErrorResponse::UnprocessableEntity {
                    message: "Network tokenization is not enabled for this profile".to_string(),
                }
                .into());
            }
            let network_token_requestor_ref_id = payment_method
                .network_token_requestor_reference_id
                .clone()
                .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
                    message: "NetworkTokenRequestorReferenceId is not present".to_string(),
                })?;

            let network_token = network_tokenization::get_token_from_tokenization_service(
                state,
                network_token_requestor_ref_id,
                payment_method,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("failed to fetch network token data from tokenization service")?;

            api::TokenDetailsResponse::NetworkTokenDetails(api::NetworkTokenDetailsResponse {
                network_token: network_token.network_token,
                network_token_exp_month: network_token.network_token_exp_month,
                network_token_exp_year: network_token.network_token_exp_year,
                cryptogram: network_token.cryptogram,
                card_issuer: network_token.card_issuer,
                card_network: network_token.card_network,
                card_type: network_token.card_type,
                card_issuing_country: network_token.card_issuing_country,
                bank_code: network_token.bank_code,
                card_holder_name: network_token.card_holder_name,
                nick_name: network_token.nick_name,
                eci: network_token.eci,
            })
        }
        common_enums::TokenDataType::SingleUseToken
        | common_enums::TokenDataType::MultiUseToken => {
            return Err(errors::ApiErrorResponse::UnprocessableEntity {
                message: "Token type not supported".to_string(),
            }
            .into());
        }
    };

    Ok(api::TokenDataResponse {
        payment_method_id: payment_method.id.clone(),
        token_type: request.token_type,
        token_details,
    })
}

#[cfg(all(feature = "v2", feature = "olap"))]
#[instrument(skip_all)]
pub async fn get_total_saved_payment_methods_for_merchant(
    state: SessionState,
    provider: domain::Provider,
) -> RouterResponse<api::TotalPaymentMethodCountResponse> {
    let total_payment_method_count = get_total_payment_method_count_core(&state, &provider).await?;

    Ok(hyperswitch_domain_models::api::ApplicationResponse::Json(
        total_payment_method_count,
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
    fn generate_response_for_session(
        self,
        customer_payment_methods: Vec<payment_methods::CustomerPaymentMethodResponseItem>,
    ) -> payment_methods::PaymentMethodListResponseForSession {
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

        payment_methods::PaymentMethodListResponseForSession {
            payment_methods_enabled: response_payment_methods,
            customer_payment_methods,
        }
    }
}

#[cfg(feature = "v2")]
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
    initiator: Option<&domain::Initiator>,
) -> CustomResult<domain::PaymentMethod, errors::ApiErrorResponse> {
    use josekit::jwe::zip::deflate::DeflateJweCompression::Def;

    let db = &*state.store;

    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            key_store,
            domain::PaymentMethod {
                customer_id: Some(customer_id.to_owned()),
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
                version: common_types::consts::API_VERSION,
                locker_fingerprint_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                network_token_requestor_reference_id: None,
                external_vault_source: None,
                external_vault_token_data: None,
                vault_type: None,
                created_by: initiator.and_then(|initiator| initiator.to_created_by()),
                last_modified_by: initiator.and_then(|initiator| initiator.to_created_by()),
                customer_details: None,
            },
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    Ok(response)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn construct_payment_method_object(
    _metadata: Option<common_utils::pii::SecretSerdeValue>,
    customer_id: &Option<id_type::GlobalCustomerId>,
    payment_method_id: id_type::GlobalPaymentMethodId,
    merchant_id: &id_type::MerchantId,
    payment_method_billing_address: Option<
        Encryptable<hyperswitch_domain_models::address::Address>,
    >,
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    payment_method_vaulting_data: Option<&domain::PaymentMethodVaultingData>,
    payment_method_type: Option<common_enums::PaymentMethod>,
    payment_method_subtype: Option<common_enums::PaymentMethodType>,
    locker_id: Option<domain::VaultId>,
    locker_fingerprint_id: Option<String>,
    external_vault_source: Option<id_type::MerchantConnectorAccountId>,
    initiator: Option<&domain::Initiator>,
) -> RouterResult<domain::PaymentMethod> {
    let current_time = common_utils::date_time::now();

    let encrypted_payment_method_data = payment_method_vaulting_data
        .map(|payment_method_vaulting_data| payment_method_vaulting_data.get_payment_methods_data())
        .async_map(|payment_method_details| async {
            let key_manager_state = &(state).into();

            cards::create_encrypted_data(key_manager_state, key_store, payment_method_details)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt Payment method data")
        })
        .await
        .transpose()?
        .map(|encoded_data| {
            encoded_data.deserialize_inner_value(|value| value.parse_value("PaymentMethodsData"))
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse Payment method data")?;

    Ok(domain::PaymentMethod {
        customer_id: Some(
            customer_id
                .clone()
                .unwrap_or(id_type::GlobalCustomerId::generate(
                    &state.conf.cell_information.id,
                )),
        ), //for guest checkout flow where customer id is not present, generated a temporary customer id to handle to conversion to diesel model.
        merchant_id: merchant_id.to_owned(),
        id: payment_method_id,
        locker_id,
        payment_method_type,
        payment_method_subtype,
        payment_method_data: encrypted_payment_method_data,
        connector_mandate_details: None,
        customer_acceptance: None,
        client_secret: None,
        status: enums::PaymentMethodStatus::New,
        network_transaction_id: None,
        created_at: current_time,
        last_modified: current_time,
        last_used_at: current_time,
        payment_method_billing_address,
        updated_by: None,
        version: common_types::consts::API_VERSION,
        locker_fingerprint_id,
        network_token_locker_id: None,
        network_token_payment_method_data: None,
        network_token_requestor_reference_id: None,
        external_vault_source,
        external_vault_token_data: None,
        vault_type: None,
        created_by: initiator.and_then(|initiator| initiator.to_created_by()),
        last_modified_by: initiator.and_then(|initiator| initiator.to_created_by()),
        customer_details: None,
    })
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_method_for_confirm(
    state: &SessionState,
    customer_id: &id_type::GlobalCustomerId,
    payment_method_id: id_type::GlobalPaymentMethodId,
    external_vault_source: Option<id_type::MerchantConnectorAccountId>,
    merchant_id: &id_type::MerchantId,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    payment_method_type: storage_enums::PaymentMethod,
    payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    encrypted_payment_method_billing_address: Option<
        Encryptable<hyperswitch_domain_models::address::Address>,
    >,
    encrypted_payment_method_data: Option<Encryptable<payment_methods::PaymentMethodsData>>,
    encrypted_external_vault_token_data: Option<
        Encryptable<payment_methods::ExternalVaultTokenData>,
    >,
    vault_type: Option<common_enums::VaultType>,
    initiator: Option<&domain::Initiator>,
) -> CustomResult<domain::PaymentMethod, errors::ApiErrorResponse> {
    let db = &*state.store;
    let current_time = common_utils::date_time::now();

    let response = db
        .insert_payment_method(
            key_store,
            domain::PaymentMethod {
                customer_id: Some(customer_id.to_owned()),
                merchant_id: merchant_id.to_owned(),
                id: payment_method_id,
                locker_id: None,
                payment_method_type: Some(payment_method_type),
                payment_method_subtype,
                payment_method_data: encrypted_payment_method_data,
                connector_mandate_details: None,
                customer_acceptance: None,
                client_secret: None,
                status: enums::PaymentMethodStatus::Inactive,
                network_transaction_id: None,
                created_at: current_time,
                last_modified: current_time,
                last_used_at: current_time,
                payment_method_billing_address: encrypted_payment_method_billing_address,
                updated_by: None,
                version: common_types::consts::API_VERSION,
                locker_fingerprint_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                network_token_requestor_reference_id: None,
                external_vault_source,
                external_vault_token_data: encrypted_external_vault_token_data,
                vault_type,
                created_by: initiator.and_then(|initiator| initiator.to_created_by()),
                last_modified_by: initiator.and_then(|initiator| initiator.to_created_by()),
                customer_details: None,
            },
            storage_scheme,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to add payment method in db")?;

    Ok(response)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub async fn get_external_vault_token(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    payment_token: String,
    vault_token: domain::VaultToken,
    payment_method_type: &storage_enums::PaymentMethod,
) -> CustomResult<domain::ExternalVaultPaymentMethodData, errors::ApiErrorResponse> {
    let db = &*state.store;

    let pm_token_data = utils::retrieve_payment_token_data(state, payment_token).await?;

    let payment_method_id = match pm_token_data {
        storage::PaymentTokenData::PermanentCard(card_token_data) => {
            card_token_data.payment_method_id
        }
        storage::PaymentTokenData::TemporaryGeneric(_) => {
            Err(errors::ApiErrorResponse::NotImplemented {
                message: errors::NotImplementedMessage::Reason(
                    "TemporaryGeneric Token not implemented".to_string(),
                ),
            })?
        }
        storage::PaymentTokenData::AuthBankDebit(_) => {
            Err(errors::ApiErrorResponse::NotImplemented {
                message: errors::NotImplementedMessage::Reason(
                    "AuthBankDebit Token not implemented".to_string(),
                ),
            })?
        }
    };

    let payment_method = db
        .find_payment_method(key_store, &payment_method_id, storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Payment method not found")?;

    let external_vault_token_data = payment_method
        .external_vault_token_data
        .clone()
        .map(Encryptable::into_inner)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Missing vault token data")?;

    let decrypted_addtional_payment_method_data = payment_method
        .payment_method_data
        .clone()
        .map(Encryptable::into_inner)
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to convert payment method data")?;

    convert_from_saved_payment_method_data(
        decrypted_addtional_payment_method_data,
        external_vault_token_data,
        vault_token,
    )
    .attach_printable("Failed to convert payment method data")
}

#[cfg(feature = "v2")]
fn convert_from_saved_payment_method_data(
    vault_additional_data: payment_methods::PaymentMethodsData,
    external_vault_token_data: payment_methods::ExternalVaultTokenData,
    vault_token: domain::VaultToken,
) -> RouterResult<domain::ExternalVaultPaymentMethodData> {
    match vault_additional_data {
        payment_methods::PaymentMethodsData::Card(card_details) => {
            Ok(domain::ExternalVaultPaymentMethodData::Card(Box::new(
                domain::ExternalVaultCard {
                    card_number: external_vault_token_data.tokenized_card_number,
                    card_exp_month: card_details.expiry_month.ok_or(
                        errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "card_details.expiry_month",
                        },
                    )?,
                    card_exp_year: card_details.expiry_year.ok_or(
                        errors::ApiErrorResponse::MissingRequiredField {
                            field_name: "card_details.expiry_year",
                        },
                    )?,
                    card_holder_name: card_details.card_holder_name,
                    bin_number: card_details.card_isin,
                    last_four: card_details.last4_digits,
                    nick_name: card_details.nick_name,
                    card_issuing_country: card_details.issuer_country,
                    card_network: card_details.card_network,
                    card_issuer: card_details.card_issuer,
                    card_type: card_details.card_type,
                    card_cvc: vault_token.card_cvc,
                    co_badged_card_data: None, // Co-badged data is not supported in external vault
                    bank_code: None,           // Bank code is not stored in external vault
                },
            )))
        }
        payment_methods::PaymentMethodsData::BankDetails(_)
        | payment_methods::PaymentMethodsData::BankDebit(_)
        | payment_methods::PaymentMethodsData::WalletDetails(_) => {
            Err(errors::ApiErrorResponse::UnprocessableEntity {
                message: "External vaulting is not supported for this payment method type"
                    .to_string(),
            }
            .into())
        }
    }
}

#[cfg(feature = "v2")]
/// Update the connector_mandate_details of the payment method with
/// new token details for the payment
fn create_connector_token_details_update(
    token_details: payment_methods::ConnectorTokenDetails,
    payment_method: &domain::PaymentMethod,
) -> hyperswitch_domain_models::mandates::CommonMandateReference {
    let connector_id = token_details.connector_id.clone();

    let reference_record =
        hyperswitch_domain_models::mandates::ConnectorTokenReferenceRecord::foreign_from(
            token_details,
        );

    let connector_token_details = payment_method.connector_mandate_details.clone();

    match connector_token_details {
        Some(mut connector_mandate_reference) => {
            connector_mandate_reference
                .insert_payment_token_reference_record(&connector_id, reference_record);

            connector_mandate_reference
        }
        None => {
            let reference_record_hash_map =
                std::collections::HashMap::from([(connector_id, reference_record)]);
            let payments_mandate_reference =
                hyperswitch_domain_models::mandates::PaymentsTokenReference(
                    reference_record_hash_map,
                );
            hyperswitch_domain_models::mandates::CommonMandateReference {
                payments: Some(payments_mandate_reference),
                payouts: None,
            }
        }
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
pub async fn create_pm_additional_data_update(
    pmd: Option<&domain::PaymentMethodVaultingData>,
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    vault_id: Option<String>,
    vault_fingerprint_id: Option<String>,
    payment_method: &domain::PaymentMethod,
    connector_token_details: Option<payment_methods::ConnectorTokenDetails>,
    network_transaction_id: Option<Secret<String>>,
    nt_data: Option<NetworkTokenPaymentMethodDetails>,
    payment_method_type: Option<common_enums::PaymentMethod>,
    payment_method_subtype: Option<common_enums::PaymentMethodType>,
    external_vault_source: Option<id_type::MerchantConnectorAccountId>,
    status: Option<storage_enums::PaymentMethodStatus>,
    initiator: Option<&domain::Initiator>,
) -> RouterResult<storage::PaymentMethodUpdate> {
    let encrypted_payment_method_data = pmd
        .map(|payment_method_vaulting_data| payment_method_vaulting_data.get_payment_methods_data())
        .async_map(|payment_method_details| async {
            let key_manager_state = &(state).into();

            cards::create_encrypted_data(key_manager_state, key_store, payment_method_details)
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to encrypt Payment method data")
        })
        .await
        .transpose()?
        .map(|encoded_data| {
            encoded_data.deserialize_inner_value(|value| {
                value.parse_value::<payment_methods::PaymentMethodsData>("PaymentMethodsData")
            })
        })
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to parse Payment method data")?
        .map(From::from);

    let connector_mandate_details_update = connector_token_details
        .map(|connector_token| {
            create_connector_token_details_update(connector_token, payment_method)
        })
        .map(From::from);

    let pm_update = storage::PaymentMethodUpdate::GenericUpdate {
        // A new payment method is created with inactive state
        // It will be marked active after payment succeeds
        status,
        locker_id: vault_id,
        payment_method_type_v2: payment_method_type,
        payment_method_subtype,
        payment_method_data: encrypted_payment_method_data,
        network_token_requestor_reference_id: nt_data
            .clone()
            .map(|data| data.network_token_requestor_reference_id),
        network_token_locker_id: nt_data.clone().map(|data| data.network_token_locker_id),
        network_token_payment_method_data: nt_data.map(|data| data.network_token_pmd.into()),
        connector_mandate_details: connector_mandate_details_update,
        locker_fingerprint_id: vault_fingerprint_id,
        external_vault_source,
        network_transaction_id,
        last_modified_by: initiator
            .and_then(|initiator| initiator.to_created_by())
            .map(|last_modified_by| last_modified_by.to_string()),
    };

    Ok(pm_update)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method_internal(
    state: &SessionState,
    pmd: &domain::PaymentMethodVaultingData,
    platform: &domain::Platform,
    existing_vault_id: Option<domain::VaultId>,
    fingerprint_id_from_vault: String,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<pm_types::AddVaultResponse> {
    let db = &*state.store;

    let mut resp_from_vault =
        vault::add_payment_method_to_vault(state, platform, pmd, existing_vault_id, customer_id)
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to add payment method in vault")?;

    // add fingerprint_id to the response
    resp_from_vault.fingerprint_id = Some(fingerprint_id_from_vault);

    Ok(resp_from_vault)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method_external(
    state: &SessionState,
    pmd: &domain::PaymentMethodCustomVaultingData,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: domain::MerchantConnectorAccountTypeDetails,
) -> RouterResult<pm_types::AddVaultResponse> {
    let merchant_connector_account = match &merchant_connector_account {
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(mca) => {
            Ok(mca.as_ref())
        }
        domain::MerchantConnectorAccountTypeDetails::MerchantConnectorDetails(_) => {
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("MerchantConnectorDetails not supported for vault operations"))
        }
    }?;

    let router_data = core_utils::construct_vault_router_data(
        state,
        merchant_account.get_id(),
        merchant_connector_account,
        Some(pmd.clone()),
        None,
        None,
        None,
    )
    .await?;

    let mut old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault insert api call",
        )?;

    let connector_name = merchant_connector_account.get_connector_name_as_string(); // always get the connector name from this call

    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    access_token::create_access_token(
        state,
        &connector_data,
        merchant_account,
        &mut old_router_data,
    )
    .await?;

    if old_router_data.response.is_ok() {
        let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
            ExternalVaultInsertFlow,
            types::VaultRequestData,
            types::VaultResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data_resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &old_router_data,
            payments_core::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_vault_failed_response()?;

        get_vault_response_for_insert_payment_method_data(router_data_resp)
    } else {
        logger::error!(
            "Error vaulting payment method: {:?}",
            old_router_data.response
        );
        Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to create access token for external vault"))
    }
}

pub fn get_payment_method_custom_data(
    payment_method_vaulting_data: hyperswitch_domain_models::vault::PaymentMethodVaultingData,
    fields_to_tokenize: Option<Vec<diesel_models::business_profile::VaultTokenField>>,
) -> RouterResult<hyperswitch_domain_models::vault::PaymentMethodCustomVaultingData> {
    match fields_to_tokenize {
        Some(fields) => {
            let keys_set: Vec<String> = fields
                .iter()
                .map(|field| field.token_type.to_string())
                .collect();

            if keys_set.is_empty() {
                // edge case where no token to vault is present
                Ok(payment_method_vaulting_data.try_into()?)
            } else {
                match payment_method_vaulting_data {
                    hyperswitch_domain_models::vault::PaymentMethodVaultingData::Card(card_details) => {
                        let mut json_data = serde_json::to_value(card_details)
                            .map_err(|_| {
                                logger::error!("Error Parsing the CardDetail to Value");
                                errors::ApiErrorResponse::InternalServerError
                            })?
                            .as_object()
                            .cloned()
                            .ok_or(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to parse Value to Object")?;

                        json_data.retain(|key, _value| keys_set.contains(key));

                        let custom_card_detail: hyperswitch_domain_models::vault::CardCustomData = serde_json::from_value(
                            serde_json::Value::Object(json_data)
                        )
                            .map_err(|_| {
                                logger::error!("Error Parsing the Value to CardCustomData");
                                errors::ApiErrorResponse::InternalServerError
                            })?;
                        Ok(hyperswitch_domain_models::vault::PaymentMethodCustomVaultingData::CardData(custom_card_detail))
                    }
                    hyperswitch_domain_models::vault::PaymentMethodVaultingData::NetworkToken(network_token_details) => {
                        let mut json_data = serde_json::to_value(network_token_details)
                            .map_err(|_| {
                                logger::error!("Error Parsing the NetworkTokenDetails to Value");
                                errors::ApiErrorResponse::InternalServerError
                            })?
                            .as_object()
                            .cloned()
                            .ok_or(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Failed to parse Value to Object")?;

                        json_data.retain(|key, _value| keys_set.contains(key));

                        let custom_network_token_detail: hyperswitch_domain_models::vault::NetworkTokenCustomData = serde_json::from_value(
                            serde_json::Value::Object(json_data)
                        )
                            .map_err(|_| {
                                logger::error!("Error Parsing the Value to NetworkTokenCustomData");
                                errors::ApiErrorResponse::InternalServerError
                            })?;
                        Ok(hyperswitch_domain_models::vault::PaymentMethodCustomVaultingData::NetworkTokenData(custom_network_token_detail))
                    }
                    hyperswitch_domain_models::vault::PaymentMethodVaultingData::CardNumber(_) => {
                        Err(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unexpected Behaviour, Card Number variant is not supported for Custom Tokenization")?
                    }
                    #[cfg(feature = "v1")]
                    hyperswitch_domain_models::vault::PaymentMethodVaultingData::BankDebit(_) => {
                        Err(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("Unexpected Behaviour, Bank Debit variant is not supported for Custom Tokenization")?
                    }
                }
            }
        }
        // default case, populate data one to one
        None => Ok(payment_method_vaulting_data.try_into()?),
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all)]
pub async fn vault_payment_method_external_v1(
    state: &SessionState,
    pmd: &hyperswitch_domain_models::vault::PaymentMethodCustomVaultingData,
    merchant_account: &domain::MerchantAccount,
    merchant_connector_account: hyperswitch_domain_models::merchant_connector_account::MerchantConnectorAccount,
    should_generate_multiple_tokens: Option<bool>,
) -> RouterResult<pm_types::AddVaultResponse> {
    let router_data = core_utils::construct_vault_router_data(
        state,
        merchant_account.get_id(),
        &merchant_connector_account,
        Some(pmd.clone()),
        None,
        None,
        should_generate_multiple_tokens,
    )
    .await?;

    let mut old_router_data = VaultConnectorFlowData::to_old_router_data(router_data)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Cannot construct router data for making the external vault insert api call",
        )?;

    let connector_name = merchant_connector_account.get_connector_name_as_string();

    let connector_data = api::ConnectorData::get_external_vault_connector_by_name(
        &state.conf.connectors,
        connector_name,
        api::GetToken::Connector,
        Some(merchant_connector_account.get_id()),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to get the connector data")?;

    access_token::create_access_token(
        state,
        &connector_data,
        merchant_account,
        &mut old_router_data,
    )
    .await?;

    if old_router_data.response.is_ok() {
        let connector_integration: services::BoxedVaultConnectorIntegrationInterface<
            ExternalVaultInsertFlow,
            types::VaultRequestData,
            types::VaultResponseData,
        > = connector_data.connector.get_connector_integration();

        let router_data_resp = services::execute_connector_processing_step(
            state,
            connector_integration,
            &old_router_data,
            payments_core::CallConnectorAction::Trigger,
            None,
            None,
        )
        .await
        .to_vault_failed_response()?;

        get_vault_response_for_insert_payment_method_data(router_data_resp)
    } else {
        logger::error!(
            "Error vaulting payment method: {:?}",
            old_router_data.response
        );
        Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to create access token for external vault"))
    }
}

pub fn get_vault_response_for_insert_payment_method_data<F>(
    router_data: VaultRouterData<F>,
) -> RouterResult<pm_types::AddVaultResponse> {
    match router_data.response {
        Ok(response) => match response {
            types::VaultResponseData::ExternalVaultInsertResponse {
                connector_vault_id,
                fingerprint_id,
            } => {
                #[cfg(feature = "v2")]
                let vault_id = domain::VaultId::generate(connector_vault_id.get_single_vault_id()?);
                #[cfg(not(feature = "v2"))]
                let vault_id = connector_vault_id;

                Ok(pm_types::AddVaultResponse {
                    vault_id,
                    fingerprint_id: Some(fingerprint_id),
                    entity_id: None,
                })
            }
            types::VaultResponseData::ExternalVaultRetrieveResponse { .. }
            | types::VaultResponseData::ExternalVaultDeleteResponse { .. }
            | types::VaultResponseData::ExternalVaultCreateResponse { .. } => {
                Err(report!(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Invalid Vault Response"))
            }
        },
        Err(err) => {
            logger::error!("Error vaulting payment method: {:?}", err);
            Err(report!(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to vault payment method"))
        }
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method(
    state: &SessionState,
    pmd: &domain::PaymentMethodVaultingData,
    platform: &domain::Platform,
    profile: &domain::Profile,
    existing_vault_id: Option<domain::VaultId>,
    fingerprint_id_from_vault: String,
    customer_id: &id_type::GlobalCustomerId,
) -> RouterResult<(
    pm_types::AddVaultResponse,
    Option<id_type::MerchantConnectorAccountId>,
)> {
    let is_external_vault_enabled = profile.is_external_vault_enabled();

    match is_external_vault_enabled {
        true => {
            let (external_vault_source, vault_token_selector) = profile
                .external_vault_connector_details
                .clone()
                .map(|connector_details| {
                    (
                        connector_details.vault_connector_id.clone(),
                        connector_details.vault_token_selector.clone(),
                    )
                })
                .ok_or(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("mca_id not present for external vault")?;

            let merchant_connector_account =
                domain::MerchantConnectorAccountTypeDetails::MerchantConnectorAccount(Box::new(
                    payments_core::helpers::get_merchant_connector_account_v2(
                        state,
                        platform.get_processor(),
                        Some(&external_vault_source),
                    )
                    .await
                    .attach_printable(
                        "failed to fetch merchant connector account for external vault insert",
                    )?,
                ));

            let payment_method_custom_data =
                get_payment_method_custom_data(pmd.clone(), vault_token_selector)?;

            vault_payment_method_external(
                state,
                &payment_method_custom_data,
                platform.get_provider().get_account(),
                merchant_connector_account,
            )
            .await
            .map(|value| (value, Some(external_vault_source)))
        }
        false => vault_payment_method_internal(
            state,
            pmd,
            platform,
            existing_vault_id,
            fingerprint_id_from_vault,
            customer_id,
        )
        .await
        .map(|value| (value, None)),
    }
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn vault_payment_method_in_volatile_storage(
    state: &SessionState,
    pmd: &domain::PaymentMethodVaultingData,
    platform: &domain::Platform,
    _profile: &domain::Profile,
    _existing_vault_id: Option<domain::VaultId>,
    customer_id: &Option<id_type::GlobalCustomerId>,
) -> RouterResult<(
    pm_types::AddVaultResponse,
    Option<id_type::MerchantConnectorAccountId>,
)> {
    let vault_id = domain::VaultId::generate(generate_id(consts::ID_LENGTH, "vault"));
    let merchant_key_store = platform.get_provider().get_key_store();

    let encrypted_payload: Encryption =
        cards::create_encrypted_data(&(state).into(), merchant_key_store, pmd.clone())
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to encrypt Payment method vaulting data")?
            .into();

    let redis_connection = state
        .store
        .get_redis_conn()
        .map_err(Into::<errors::StorageError>::into)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    logger::info!("Storing payment method vaulting data in redis");
    redis_connection
        .serialize_and_set_key_with_expiry(
            &vault_id.get_string_repr().into(),
            encrypted_payload,
            consts::DEFAULT_PAYMENT_METHOD_STORE_TTL,
        )
        .await
        .map_err(Into::<errors::StorageError>::into)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert payment method vaulting data to redis")?;

    Ok((
        pm_types::AddVaultResponse {
            entity_id: customer_id.clone(),
            vault_id,
            fingerprint_id: None,
        },
        None,
    ))
}

#[cfg(feature = "v2")]
fn get_pm_list_context(
    payment_method_type: enums::PaymentMethod,
    payment_method: &domain::PaymentMethod,
    is_payment_associated: bool,
    storage_type: enums::StorageType,
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
                        payment_method.get_id().clone(),
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
                        storage_type,
                    ),
                ),
            })
        }
        Some(payment_methods::PaymentMethodsData::BankDetails(bank_details)) => {
            let get_bank_account_token_data =
                || -> CustomResult<payment_methods::BankAccountTokenData,errors::ApiErrorResponse> {
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
        Some(payment_methods::PaymentMethodsData::BankDebit(_))
        | Some(payment_methods::PaymentMethodsData::WalletDetails(_))
        | None => Some(PaymentMethodListContext::TemporaryToken {
            token_data: is_payment_associated.then_some(
                storage::PaymentTokenData::temporary_generic(generate_id(
                    consts::ID_LENGTH,
                    "token",
                )),
            ),
        }),
    };

    Ok(payment_method_retrieval_context)
}

#[cfg(feature = "v2")]
fn get_pm_list_token_data(
    payment_method_type: enums::PaymentMethod,
    payment_method: &domain::PaymentMethod,
    storage_type: enums::StorageType,
) -> Result<Option<storage::PaymentTokenData>, error_stack::Report<errors::ApiErrorResponse>> {
    let pm_list_context =
        get_pm_list_context(payment_method_type, payment_method, true, storage_type)?
            .get_required_value("PaymentMethodListContext")?;

    match pm_list_context {
        PaymentMethodListContext::Card {
            card_details: _,
            token_data,
        } => Ok(token_data),
        PaymentMethodListContext::Bank { token_data } => Ok(token_data),
        PaymentMethodListContext::BankTransfer {
            bank_transfer_details: _,
            token_data,
        } => Ok(token_data),
        PaymentMethodListContext::TemporaryToken { token_data } => Ok(token_data),
    }
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn list_payment_methods_core(
    state: &SessionState,
    provider: &domain::Provider,
    customer_id: &id_type::GlobalCustomerId,
    include_new: bool,
) -> RouterResult<payment_methods::CustomerPaymentMethodsListResponse> {
    let db = &*state.store;

    let statuses = if include_new {
        vec![
            common_enums::PaymentMethodStatus::Active,
            common_enums::PaymentMethodStatus::New,
        ]
    } else {
        vec![common_enums::PaymentMethodStatus::Active]
    };
    let saved_payment_methods = db
        .find_payment_method_by_global_customer_id_merchant_id_statuses(
            provider.get_key_store(),
            customer_id,
            provider.get_account().get_id(),
            statuses,
            None,
            provider.get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let customer_payment_methods = saved_payment_methods
        .into_iter()
        .map(ForeignTryFrom::foreign_try_from)
        .collect::<Result<Vec<payment_methods::PaymentMethodResponseItem>, _>>()
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let response = payment_methods::CustomerPaymentMethodsListResponse {
        customer_payment_methods,
    };

    Ok(response)
}

#[cfg(all(feature = "v2", feature = "oltp"))]
pub async fn list_customer_payment_methods_core(
    state: &SessionState,
    provider: &domain::Provider,
    customer_id: &id_type::GlobalCustomerId,
    include_new: bool,
) -> RouterResult<Vec<payment_methods::CustomerPaymentMethodResponseItem>> {
    let db = &*state.store;
    let statuses = if include_new {
        vec![
            common_enums::PaymentMethodStatus::Active,
            common_enums::PaymentMethodStatus::New,
        ]
    } else {
        vec![common_enums::PaymentMethodStatus::Active]
    };
    let saved_payment_methods = db
        .find_payment_method_by_global_customer_id_merchant_id_statuses(
            provider.get_key_store(),
            customer_id,
            provider.get_account().get_id(),
            statuses,
            None,
            provider.get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let mut customer_payment_methods = Vec::new();

    let payment_method_results: Result<Vec<_>, error_stack::Report<errors::ApiErrorResponse>> =
        saved_payment_methods
            .into_iter()
            .map(|pm| async move {
                let parent_payment_method_token = generate_id(consts::ID_LENGTH, "token");

                // For payment methods that are active we should always have the payment method type
                let payment_method_type = pm
                    .payment_method_type
                    .get_required_value("payment_method_type")?;

                let intent_fulfillment_time = common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME;

                let token_data = get_pm_list_token_data(
                    payment_method_type,
                    &pm,
                    // storage type is hardcoded to Persistent as we are fetching saved payment methods from the database
                    enums::StorageType::Persistent,
                )?;

                if let Some(token_data) = token_data {
                    pm_routes::ParentPaymentMethodToken::create_key_for_token(
                        &parent_payment_method_token,
                    )
                    .insert(intent_fulfillment_time, token_data, state)
                    .await?;

                    let final_pm = api::CustomerPaymentMethodResponseItem::foreign_try_from((
                        pm,
                        parent_payment_method_token,
                    ))
                    .change_context(errors::ApiErrorResponse::InternalServerError)
                    .attach_printable("Failed to convert payment method to response format")?;

                    Ok(Some(final_pm))
                } else {
                    Ok(None)
                }
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .try_collect::<Vec<_>>()
            .await;

    customer_payment_methods.extend(payment_method_results?.into_iter().flatten());

    Ok(customer_payment_methods)
}

#[cfg(all(feature = "v2", feature = "olap"))]
pub async fn get_total_payment_method_count_core(
    state: &SessionState,
    provider: &domain::Provider,
) -> RouterResult<api::TotalPaymentMethodCountResponse> {
    let db = &*state.store;

    let total_count = db
        .get_payment_method_count_by_merchant_id_status(
            provider.get_account().get_id(),
            common_enums::PaymentMethodStatus::Active,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to get total payment method count")?;

    let response = api::TotalPaymentMethodCountResponse { total_count };

    Ok(response)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn retrieve_payment_method(
    state: SessionState,
    pm: api::PaymentMethodId,
    profile: domain::Profile,
    platform: domain::Platform,
    api_key_type: enums::ApiKeyType,
    fetch_raw_detail_query_param: bool,
) -> RouterResponse<api::PaymentMethodResponse> {
    let db = state.store.as_ref();

    // 1. Resolve parent token (if any) -> storage type & optional token data
    let (storage_type, card_token_data_opt) =
        resolve_storage_type_from_token(&state, &pm.payment_method_id).await?;

    // 2. Fetch payment method record based on resolved storage type
    let (storage_type, payment_method) =
        fetch_payment_method_by_storage(&state, &platform, &pm, storage_type, card_token_data_opt)
            .await
            .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
            .attach_printable("Failed to fetch payment method by storage")?;
    when(
        payment_method.status == common_enums::PaymentMethodStatus::Inactive,
        || {
            Err(report!(errors::ApiErrorResponse::PaymentMethodNotFound)
                .attach_printable("Payment method is inactive"))
        },
    )?;

    let raw_payment_method_fetch_access = get_raw_payment_method_data_fetch_access(
        db,
        platform.get_provider().get_account().get_id(),
        api_key_type,
        fetch_raw_detail_query_param,
    )
    .await
    .attach_printable("Failed to get raw payment method fetch access")?;

    let single_use_token_in_cache = get_single_use_token_from_store(
        &state.clone(),
        domain::SingleUseTokenKey::store_key(&payment_method.id.clone()),
    )
    .await
    .unwrap_or_default();

    let card_cvc_expiry = vault::retrieve_key_and_ttl_for_cvc_from_payment_method_id(
        &state,
        payment_method.id.to_owned(),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to retrieve cvc from redis")
    .ok();

    let raw_payment_method_data = raw_payment_method_fetch_access
        .get_raw_payment_method_data(&state, &platform, &profile, &payment_method, storage_type)
        .await
        .attach_printable("Failed to get raw payment method data")?
        .and_then(|data| data.convert_to_raw_payment_method_data());
    let billing = payment_method
        .payment_method_billing_address
        .clone()
        .map(|billing| billing.into_inner())
        .map(From::from);

    transformers::generate_payment_method_response(
        &payment_method,
        &single_use_token_in_cache,
        storage_type,
        card_cvc_expiry.map(|time| {
            payment_methods::CardCVCTokenStorageDetails::generate_expiry_timestamp(time)
        }),
        payment_method.customer_id.clone(),
        raw_payment_method_data,
        billing,
        None,
    )
    .map(services::ApplicationResponse::Json)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
async fn fetch_payment_method_by_storage(
    state: &SessionState,
    platform: &domain::Platform,
    pm_incoming: &api::PaymentMethodId,
    storage_type: common_enums::StorageType,
    card_token_data_opt: Option<storage::CardTokenData>,
) -> RouterResult<(common_enums::StorageType, domain::PaymentMethod)> {
    let db = state.store.as_ref();
    match storage_type {
        common_enums::StorageType::Volatile => {
            logger::debug!("Fetching volatile payment method");
            // When it is Volatile storage payment token data will be always present in redis
            let token_data = card_token_data_opt
                .as_ref()
                .ok_or(report!(errors::ApiErrorResponse::PaymentMethodNotFound))
                .attach_printable("Failed to get card token data for volatile storage")?;

            let volatile_payment_method = fetch_volatile_payment_method_record(
                state,
                platform.get_provider().get_key_store(),
                token_data.payment_method_id.get_string_repr(),
            )
            .await
            .attach_printable("Failed to get volatile payment method record")?;
            Ok((storage_type, volatile_payment_method))
        }
        common_enums::StorageType::Persistent => {
            logger::debug!("Fetching persistent payment method with fallback");
            // In S2S calls, id passed in the request could be payment method id as well
            // If a temporary token is passed after the redis expiration it will also be treated as
            // a persistent GlobalPaymentMethodId, but for temp tokens GlobalPaymentMethodId will fail
            let pm_id = match card_token_data_opt {
                Some(data) => data.payment_method_id.clone(),
                None => id_type::GlobalPaymentMethodId::generate_from_string(
                    pm_incoming.payment_method_id.clone(),
                )
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Unable to generate GlobalPaymentMethodId")?,
            };

            fetch_payment_method_with_fallback(state, platform, &pm_id, storage_type)
                .await
                .attach_printable("Failed to get payment method with fallback")
        }
    }
}

#[cfg(feature = "v2")]
pub async fn fetch_payment_method_with_fallback(
    state: &SessionState,
    platform: &domain::Platform,
    pm_id: &id_type::GlobalPaymentMethodId,
    storage_type: common_enums::StorageType,
) -> RouterResult<(common_enums::StorageType, domain::PaymentMethod)> {
    let volatile_payment_method = fetch_volatile_payment_method_record(
        state,
        platform.get_provider().get_key_store(),
        pm_id.get_string_repr(),
    )
    .await
    .attach_printable("Failed to get volatile payment method record");

    match volatile_payment_method {
        Ok(payment_method) => Ok((common_enums::StorageType::Volatile, payment_method)),
        Err(err) => {
            logger::warn!("Volatile payment method not found, falling back to persistent storage",);

            when(
                !matches!(
                    err.current_context(),
                    errors::ApiErrorResponse::GenericNotFoundError { .. }
                ),
                || Err(err),
            )?;

            logger::debug!("Redis lookup failed, falling back to DB");

            let persistent_payment_method =
                fetch_payment_method(state, platform.get_provider(), pm_id)
                    .await
                    .attach_printable("Failed to get payment method record from DB")?;

            Ok((storage_type, persistent_payment_method))
        }
    }
}

#[cfg(feature = "v2")]
async fn fetch_volatile_payment_method_record(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    pm_id: &str,
) -> RouterResult<domain::PaymentMethod> {
    let redis_conn = state
        .store
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get redis connection")?;

    let payment_method = redis_conn
        .get_and_deserialize_key::<diesel_models::PaymentMethod>(&pm_id.into(), "PaymentMethod")
        .await
        .map_err(|e| error_stack::report!(storage_impl::StorageError::from(e)))
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "Payment method token either expired or does not exist".to_string(),
        })?;

    let keymanager_state = &state.into();

    let domain_payment_method = domain::PaymentMethod::convert_back(
        keymanager_state,
        payment_method,
        key_store.key.get_inner(),
        key_store.merchant_id.clone().into(),
    )
    .await
    .change_context(errors::StorageError::EncryptionError)
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    Ok(domain_payment_method)
}

#[cfg(feature = "v2")]
async fn resolve_storage_type_from_token(
    state: &SessionState,
    token: &String,
) -> RouterResult<(common_enums::StorageType, Option<storage::CardTokenData>)> {
    // Try redis lookup for parent token
    let token_data_res = pm_routes::ParentPaymentMethodToken::create_key_for_token(token)
        .get_data_for_token(state)
        .await;

    match token_data_res {
        Ok(token_data) => {
            // only Permanent/PermanentCard token variants carry storage type info relevant for saved PMs
            match token_data {
                storage::PaymentTokenData::PermanentCard(card) => {
                    let storage_type = card.storage_type;
                    Ok((storage_type, Some(card)))
                }
                // Any token kind other than PermanentCard is invalid for retrieving a saved payment method.
                // Only PermanentCard is currently supported; all other enum variants should be removed.
                storage::PaymentTokenData::TemporaryGeneric(_)
                | storage::PaymentTokenData::AuthBankDebit(_) => {
                    Err(report!(errors::ApiErrorResponse::PaymentMethodNotFound))
                }
            }
        }
        Err(err) => {
            // If token not found in redis treat incoming id as a persistent PM id as it could have been obtained from a S2S payment method retrieve call
            // Or it would be an MIT case
            if matches!(
                err.current_context(),
                errors::ApiErrorResponse::GenericNotFoundError { .. }
            ) {
                Ok((common_enums::StorageType::Persistent, None))
            } else {
                Err(err)
            }
        }
    }
}

#[derive(
    Clone, Copy, Debug, strum::Display, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub enum RawPaymentMethodFetchAccess {
    Allowed,
    Denied,
}

#[cfg(feature = "v2")]
impl RawPaymentMethodFetchAccess {
    pub async fn get_raw_payment_method_data(
        &self,
        state: &SessionState,
        platform: &domain::Platform,
        profile: &domain::Profile,
        payment_method: &domain::PaymentMethod,
        storage_type: common_enums::StorageType,
    ) -> RouterResult<Option<hyperswitch_domain_models::vault::PaymentMethodVaultingData>> {
        match self {
            Self::Denied => {
                logger::debug!("Raw payment method fetch access denied");
                Ok(None)
            }

            Self::Allowed => {
                let vault_data = vault::retrieve_payment_method_data_from_storage(
                    state,
                    platform,
                    profile,
                    payment_method,
                    storage_type,
                )
                .await
                .change_context(errors::ApiErrorResponse::InternalServerError)
                .attach_printable("Failed to retrieve payment method from vault")?
                .data;

                let payment_method_vault_data = vault_data
                    .populated_payment_methods_data_and_get_payment_method_vaulting_data(
                        payment_method.payment_method_data.as_ref(),
                    )
                    .attach_printable(
                        "Failed to get card details for payment method vaulting data",
                    )?;
                Ok(Some(payment_method_vault_data))
            }
        }
    }
}

pub async fn get_raw_payment_method_data_fetch_access(
    db: &dyn StorageInterface,
    merchant_id: &id_type::MerchantId,
    api_key_type: enums::ApiKeyType,
    fetch_raw_detail_query_param: bool,
) -> RouterResult<RawPaymentMethodFetchAccess> {
    // If query param not set, never allowed to fetch raw payment method details
    let fetch_access = match fetch_raw_detail_query_param {
        true => RawPaymentMethodFetchAccess::Allowed,
        false => RawPaymentMethodFetchAccess::Denied,
    };

    match api_key_type {
        // Internal API keys always allowed
        enums::ApiKeyType::Internal => Ok(fetch_access),

        // External API keys allowed only via org-level config
        // This supports cases where a PCI-compliant entity needs to retrieve raw payment method details.
        enums::ApiKeyType::External => {
            let config = db
                .find_config_by_key_unwrap_or(
                    &merchant_id.should_return_raw_payment_method_details_key(),
                    Some("false".to_string()),
                )
                .await;

            match config {
                Ok(conf) if conf.config.eq_ignore_ascii_case("true") => Ok(fetch_access),
                Ok(_) => Ok(RawPaymentMethodFetchAccess::Denied),
                Err(error) => {
                    router_env::logger::error!(
                        ?error,
                        "Failed to fetch raw payment method details config"
                    );
                    Ok(RawPaymentMethodFetchAccess::Denied)
                }
            }
        }
    }
}

// TODO: When we separate out microservices, this function will be an endpoint in payment_methods
#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn update_payment_method_status_internal(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
    storage_scheme: enums::MerchantStorageScheme,
    status: enums::PaymentMethodStatus,
    payment_method_id: &id_type::GlobalPaymentMethodId,
    initiator: Option<&domain::Initiator>,
) -> RouterResult<domain::PaymentMethod> {
    let db = &*state.store;

    let payment_method = db
        .find_payment_method(key_store, payment_method_id, storage_scheme)
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

    let pm_update = storage::PaymentMethodUpdate::StatusUpdate {
        status: Some(status),
        last_modified_by: initiator
            .and_then(|initiator| initiator.to_created_by())
            .map(|last_modified_by| last_modified_by.to_string()),
    };

    let updated_pm = db
        .update_payment_method(key_store, payment_method.clone(), pm_update, storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update payment method in db")?;

    Ok(updated_pm)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn update_payment_method(
    state: SessionState,
    platform: domain::Platform,
    profile: domain::Profile,
    req: api::PaymentMethodUpdate,
    payment_method_id: &id_type::GlobalPaymentMethodId,
) -> RouterResponse<api::PaymentMethodResponse> {
    let update_request = DomainPaymentMethodUpdate::from(req);

    let (response, _updated_payment_method) = Box::pin(update_payment_method_core(
        &state,
        &platform,
        &profile,
        update_request,
        payment_method_id,
        None,
        None,
    ))
    .await?;

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn update_payment_method_core(
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    request: DomainPaymentMethodUpdate,
    payment_method_id: &id_type::GlobalPaymentMethodId,
    existing_payment_method: Option<domain::PaymentMethod>,
    network_tokenization_resp: Option<NetworkTokenPaymentMethodDetails>,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    let mut handler = match existing_payment_method {
        Some(payment_method) => {
            let handler = pm_types::PaymentMethodUpdateHandler {
                platform,
                profile,
                request,
                payment_method,
                state,
            };
            handler.validate()?;
            handler
        }
        None => pm_types::PaymentMethodUpdateHandler::generate(
            state,
            platform,
            profile,
            request,
            payment_method_id,
        )
        .await
        .attach_printable("Failed to generate PaymentMethodUpdateHandler")?,
    };

    Box::pin(execute_payment_method_update_handler(
        &mut handler,
        network_tokenization_resp,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
async fn execute_payment_method_update_handler(
    handler: &mut pm_types::PaymentMethodUpdateHandler<'_>,
    network_tokenization_resp: Option<NetworkTokenPaymentMethodDetails>,
) -> RouterResult<(api::PaymentMethodResponse, domain::PaymentMethod)> {
    let card_cvc_details = handler
        .update_cvc_if_required()
        .await
        .attach_printable("Failed to update card cvc")?;

    let (vaulting_data, vaulting_resp) = handler
        .perform_vaulting_operations_if_required()
        .await
        .attach_printable("Failed to perform vaulting operations for payment method update")?;

    handler
        .update_payment_method_if_required(vaulting_data, vaulting_resp, network_tokenization_resp)
        .await
        .attach_printable("Failed to update payment method in db")?;

    let response = handler
        .generate_response(card_cvc_details)
        .attach_printable("Failed to generate response for payment method update")?;

    Ok((response, handler.payment_method.clone()))
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn delete_payment_method(
    state: SessionState,
    pm_id: api::PaymentMethodId,
    platform: domain::Platform,
    profile: domain::Profile,
) -> RouterResponse<api::PaymentMethodDeleteResponse> {
    let pm_id = id_type::GlobalPaymentMethodId::generate_from_string(pm_id.payment_method_id)
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unable to generate GlobalPaymentMethodId")?;
    let response = Box::pin(delete_payment_method_core(
        &state, pm_id, &platform, &profile,
    ))
    .await?;

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn delete_payment_method_core(
    state: &SessionState,
    pm_id: id_type::GlobalPaymentMethodId,
    platform: &domain::Platform,
    profile: &domain::Profile,
) -> RouterResult<api::PaymentMethodDeleteResponse> {
    let db = state.store.as_ref();

    let payment_method = db
        .find_payment_method(
            platform.get_provider().get_key_store(),
            &pm_id,
            platform.get_provider().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;
    let customer_id = &payment_method
        .customer_id
        .clone()
        .get_required_value("GlobalCustomerId")?;

    when(
        payment_method.status == enums::PaymentMethodStatus::Inactive,
        || Err(errors::ApiErrorResponse::PaymentMethodNotFound),
    )?;

    let _customer = db
        .find_customer_by_global_id_merchant_id(
            customer_id,
            platform.get_provider().get_account().get_id(),
            platform.get_provider().get_key_store(),
            platform.get_provider().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Customer not found for the payment method")?;

    delete_payment_method_by_record(db, state, platform, profile, payment_method).await?;

    let response = api::PaymentMethodDeleteResponse { id: pm_id };

    Ok(response)
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
pub async fn delete_payment_method_by_record(
    db: &dyn StorageInterface,
    state: &SessionState,
    platform: &domain::Platform,
    profile: &domain::Profile,
    payment_method: domain::PaymentMethod,
) -> RouterResult<()> {
    // Soft delete
    let pm_update = storage::PaymentMethodUpdate::StatusAndFingerprintUpdate {
        status: Some(enums::PaymentMethodStatus::Inactive),
        last_modified_by: platform
            .get_initiator()
            .and_then(|initiator| initiator.to_created_by())
            .map(|last_modified_by| last_modified_by.to_string()),
        locker_fingerprint_id: Some(PAYMENT_METHOD_REDACTED_FINGERPRINT_ID.to_string()),
    };

    db.update_payment_method(
        platform.get_provider().get_key_store(),
        payment_method.clone(),
        pm_update,
        platform.get_provider().get_account().storage_scheme,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update payment method in db")?;

    vault::delete_payment_method_data_from_vault(state, platform, profile, &payment_method)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete payment method from vault")?;

    Ok(())
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
    type Output = hyperswitch_domain_models::payment_methods::DecryptedPaymentMethodSession;

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
            common_utils::type_name!(hyperswitch_domain_models::payment_methods::PaymentMethodSession),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodSession::to_encryptable(
                    hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodSession {
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
        hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodSession::from_encryptable(
            batch_encrypted_data,
        )
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while encrypting payment methods session detailss")?;

        Ok(encrypted_data)
    }
}

#[cfg(feature = "v2")]
#[async_trait::async_trait]
impl EncryptableData for payment_methods::PaymentMethodsSessionUpdateRequest {
    type Output = hyperswitch_domain_models::payment_methods::DecryptedPaymentMethodSession;

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
            common_utils::type_name!(hyperswitch_domain_models::payment_methods::PaymentMethodSession),
            domain_types::CryptoOperation::BatchEncrypt(
                hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodSession::to_encryptable(
                    hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodSession {
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
        hyperswitch_domain_models::payment_methods::FromRequestEncryptablePaymentMethodSession::from_encryptable(
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
    platform: domain::Platform,
    profile: domain::Profile,
    request: payment_methods::PaymentMethodSessionRequest,
) -> RouterResponse<payment_methods::PaymentMethodSessionResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let provider = platform.get_provider();

    if let (Some(customer_id)) = &request.customer_id {
        db.find_customer_by_global_id_merchant_id(
            customer_id,
            platform.get_provider().get_account().get_id(),
            platform.get_provider().get_key_store(),
            platform.get_provider().get_account().storage_scheme,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::CustomerNotFound)
        .attach_printable("Customer not found for the payment method")?;
    }

    let payment_methods_session_id =
        id_type::GlobalPaymentMethodSessionId::generate(&state.conf.cell_information.id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Unable to generate GlobalPaymentMethodSessionId")?;

    let encrypted_data = request
        .encrypt_data(key_manager_state, platform.get_provider().get_key_store())
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
        platform.get_provider().get_account().get_id(),
        util_types::authentication::ResourceId::PaymentMethodSession(
            payment_methods_session_id.clone(),
        ),
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Unable to create client secret")?;

    let payment_method_session_domain_model =
        hyperswitch_domain_models::payment_methods::PaymentMethodSession {
            id: payment_methods_session_id,
            customer_id: request.customer_id.clone(),
            billing,
            psp_tokenization: request.psp_tokenization,
            network_tokenization: request.network_tokenization,
            tokenization_data: request.tokenization_data,
            expires_at,
            return_url: request.return_url,
            associated_payment_methods: None,
            associated_payment: None,
            associated_token_id: None,
            storage_type: request.storage_type,
            keep_alive: request.keep_alive.unwrap_or_default(),
        };

    db.insert_payment_methods_session(
        platform.get_provider().get_key_store(),
        payment_method_session_domain_model.clone(),
        expires_in,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to insert payment methods session in db")?;

    let sdk_authorization = Option::<hyperswitch_domain_models::sdk_auth::SdkAuthorization>::from(
        hyperswitch_domain_models::sdk_auth::SdkAuthorizationContext {
            platform,
            profile_id: profile.get_id().clone(),
            client_secret: client_secret.secret.clone().expose(),
            customer_id: payment_method_session_domain_model.customer_id.clone(),
            payment_method_session_id: Some(payment_method_session_domain_model.id.clone()),
        },
    );

    let response = transformers::generate_payment_method_session_response(
        payment_method_session_domain_model,
        client_secret.secret,
        sdk_authorization,
        None,
        None,
        request.storage_type,
        None,
        None,
    );

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn payment_methods_session_update(
    state: SessionState,
    provider: domain::Provider,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
    request: payment_methods::PaymentMethodsSessionUpdateRequest,
) -> RouterResponse<payment_methods::PaymentMethodSessionResponse> {
    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();

    let existing_payment_method_session_state = db
        .get_payment_methods_session(provider.get_key_store(), &payment_method_session_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to retrieve payment methods session from db")?;

    let encrypted_data = request
        .encrypt_data(key_manager_state, provider.get_key_store())
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

    let payment_method_session_domain_model =
        hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum::GeneralUpdate{
            billing: Box::new(billing),
            psp_tokenization: request.psp_tokenization,
            network_tokenization: request.network_tokenization,
            tokenization_data: request.tokenization_data,
        };

    let update_state_change = db
        .update_payment_method_session(
            provider.get_key_store(),
            &payment_method_session_id,
            payment_method_session_domain_model,
            existing_payment_method_session_state.clone(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update payment methods session in db")?;

    let response = transformers::generate_payment_method_session_response(
        update_state_change.clone(),
        Secret::new("CLIENT_SECRET_REDACTED".to_string()),
        None, // sdk_authorization is not returned for non-create flows
        None, // TODO: send associated payments response based on the expandable param
        None,
        update_state_change.storage_type,
        None,
        None,
    );

    Ok(services::ApplicationResponse::Json(response))
}
#[cfg(feature = "v2")]
pub async fn payment_methods_session_retrieve(
    state: SessionState,
    provider: domain::Provider,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
) -> RouterResponse<payment_methods::PaymentMethodSessionResponse> {
    let db = state.store.as_ref();

    let payment_method_session_domain_model = db
        .get_payment_methods_session(provider.get_key_store(), &payment_method_session_id)
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to retrieve payment methods session from db")?;

    // Only one associated payment method is supported for now
    let associated_pm_token = payment_method_session_domain_model
        .associated_payment_methods
        .as_ref()
        .and_then(|payment_methods| {
            payment_methods.iter().map(|pm| match &pm.payment_method_token {
                common_types::payment_methods::AssociatedPaymentMethodTokenType::PaymentMethodSessionToken(token) => token.clone(),
            }).next()
        });

    let expiry = associated_pm_token
        .async_map(|pm_token| {
            utils::retrieve_payment_method_id_from_payment_method_token_data(&state, pm_token)
        })
        .await
        .transpose()
        .ok()
        .flatten()
        .async_map(|pm_id| {
            vault::retrieve_key_and_ttl_for_cvc_from_payment_method_id(&state, pm_id.to_owned())
        })
        .await
        .transpose()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve cvc from redis")
        .ok()
        .flatten();

    let response = transformers::generate_payment_method_session_response(
        payment_method_session_domain_model.clone(),
        Secret::new("CLIENT_SECRET_REDACTED".to_string()),
        None, // sdk_authorization is not returned for non-create flows
        None, // TODO: send associated payments response based on the expandable param
        None,
        payment_method_session_domain_model.storage_type,
        expiry.map(|time| {
            payment_methods::CardCVCTokenStorageDetails::generate_expiry_timestamp(time)
        }),
        None,
    );

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn payment_methods_session_update_payment_method(
    state: SessionState,
    platform: domain::Platform,
    profile: domain::Profile,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
    request: payment_methods::PaymentMethodSessionUpdateSavedPaymentMethod,
) -> RouterResponse<payment_methods::PaymentMethodSessionResponse> {
    let db = state.store.as_ref();

    // Validate if the session still exists
    let payment_method_session = db
        .get_payment_methods_session(
            platform.get_provider().get_key_store(),
            &payment_method_session_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to retrieve payment methods session from db")?;

    // Get the associated_pm_token_details for the payment_method_token from the request
    payment_method_session
        .associated_payment_methods
        .as_ref()
        .and_then(|payment_methods| {
            payment_methods.iter().find(|pm| match &pm.payment_method_token {
                common_types::payment_methods::AssociatedPaymentMethodTokenType::PaymentMethodSessionToken(token) => token == &request.payment_method_token
            })
        })
        .ok_or(errors::ApiErrorResponse::GenericNotFoundError {
            message: "No associated payment method found in the session".to_string(),
        })?;

    let payment_method_id = utils::retrieve_payment_method_id_from_payment_method_token_data(
        &state,
        request.payment_method_token.clone(),
    )
    .await
    .attach_printable("Failed to retrieve payment method id from payment method token data")?;

    // Insert the token as the first element in the associated payment methods list
    let mut tokens =  payment_method_session
        .associated_payment_methods
        .clone()
        .map(|tokens| tokens.into_iter().filter(|token| match &token.payment_method_token {
            common_types::payment_methods::AssociatedPaymentMethodTokenType::PaymentMethodSessionToken(token) => token != &request.payment_method_token
        }).collect::<Vec<_>>());

    tokens.as_mut().map(|tokens| tokens.insert(0, common_types::payment_methods::AssociatedPaymentMethods {
            payment_method_token: common_types::payment_methods::AssociatedPaymentMethodTokenType::PaymentMethodSessionToken(request.payment_method_token.clone()),
        }));

    // Update payment method session with associated payment methods
    let update_payment_method_session = hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum::UpdateAssociatedPaymentMethods {
        associated_payment_methods: tokens
    };

    let updated_payment_method_session = db
        .update_payment_method_session(
            platform.get_provider().get_key_store(),
            &payment_method_session_id,
            update_payment_method_session,
            payment_method_session,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable(
            "Failed to update payment method session with associated payment methods",
        )?;

    let update_request =
        DomainPaymentMethodUpdate::from(request.payment_method_update_request.clone());

    let (update_response, _updated_payment_method) = Box::pin(update_payment_method_core(
        &state,
        &platform,
        &profile,
        update_request,
        &payment_method_id,
        None,
        None,
    ))
    .await
    .attach_printable("Failed to update saved payment method")?;

    let response = transformers::generate_payment_method_session_response(
        updated_payment_method_session.clone(),
        Secret::new("CLIENT_SECRET_REDACTED".to_string()),
        None, // sdk_authorization is not returned for non-create flows
        None, // TODO: send associated payments response based on the expandable param
        None,
        updated_payment_method_session.storage_type,
        update_response.card_cvc_token_storage,
        update_response.payment_method_data.clone(),
    );

    Ok(services::ApplicationResponse::Json(response))
}

#[cfg(feature = "v2")]
pub async fn payment_methods_session_delete_payment_method(
    state: SessionState,
    platform: domain::Platform,
    profile: domain::Profile,
    pm_token: String,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
) -> RouterResponse<payment_methods::PaymentMethodDeleteSessionResponse> {
    let db = state.store.as_ref();

    // Validate if the session still exists
    let payment_method_session = db
        .get_payment_methods_session(
            platform.get_provider().get_key_store(),
            &payment_method_session_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to retrieve payment methods session from db")?;

    let payment_method_id =
        utils::retrieve_payment_method_id_from_payment_method_token_data(&state, pm_token.clone())
            .await
            .attach_printable(
                "Failed to retrieve payment method id from payment method token data",
            )?;

    Box::pin(delete_payment_method_core(
        &state,
        payment_method_id,
        &platform,
        &profile,
    ))
    .await
    .attach_printable("Failed to delete saved payment method")?;

    let tokens =  payment_method_session
        .associated_payment_methods
        .clone()
        .map(|tokens| tokens.into_iter().filter(|token| match &token.payment_method_token {
            common_types::payment_methods::AssociatedPaymentMethodTokenType::PaymentMethodSessionToken(token) => token != &pm_token
        }).collect::<Vec<_>>());

    // Update payment method session with associated payment methods
    let update_payment_method_session = hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum::UpdateAssociatedPaymentMethods {
        associated_payment_methods: tokens
    };

    db.update_payment_method_session(
        platform.get_provider().get_key_store(),
        &payment_method_session_id,
        update_payment_method_session,
        payment_method_session,
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to update payment method session with associated payment methods")?;

    Ok(services::ApplicationResponse::Json(
        payment_methods::PaymentMethodDeleteSessionResponse {
            payment_method_token: pm_token,
        },
    ))
}

#[cfg(feature = "v2")]
fn construct_zero_auth_payments_request(
    confirm_request: &payment_methods::PaymentMethodSessionConfirmRequest,
    payment_method_session: &hyperswitch_domain_models::payment_methods::PaymentMethodSession,
    payment_method: &payment_methods::PaymentMethodResponse,
    payment_method_subtype: Option<storage_enums::PaymentMethodType>,
) -> RouterResult<api_models::payments::PaymentsRequest> {
    use api_models::payments;

    Ok(payments::PaymentsRequest {
        amount_details: payments::AmountDetails::new_for_zero_auth_payment(
            common_enums::Currency::USD,
        ),
        payment_method_data: confirm_request.payment_method_data.clone(),
        payment_method_type: confirm_request.payment_method_type,
        payment_method_subtype: payment_method_subtype
            .get_required_value("payment_method_subtype")?,
        customer_id: payment_method_session.customer_id.clone(),
        customer_present: Some(enums::PresenceOfCustomerDuringPayment::Present),
        setup_future_usage: Some(common_enums::FutureUsage::OffSession),
        payment_method_id: Some(payment_method.id.clone()),
        merchant_reference_id: None,
        routing_algorithm_id: None,
        capture_method: None,
        authentication_type: None,
        // We have already passed payment method billing address
        billing: None,
        shipping: None,
        description: None,
        return_url: payment_method_session.return_url.clone(),
        apply_mit_exemption: None,
        statement_descriptor: None,
        order_details: None,
        allowed_payment_method_types: None,
        metadata: None,
        connector_metadata: None,
        feature_metadata: None,
        payment_link_enabled: None,
        payment_link_config: None,
        request_incremental_authorization: None,
        session_expiry: None,
        frm_metadata: None,
        request_external_three_ds_authentication: None,
        customer_acceptance: None,
        browser_info: None,
        force_3ds_challenge: None,
        is_iframe_redirection_enabled: None,
        merchant_connector_details: None,
        return_raw_connector_response: None,
        enable_partial_authorization: None,
        webhook_url: None,
    })
}

#[cfg(feature = "v2")]
async fn create_zero_auth_payment(
    state: SessionState,
    req_state: routes::app::ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    request: api_models::payments::PaymentsRequest,
) -> RouterResult<api_models::payments::PaymentsResponse> {
    let response = Box::pin(payments_core::payments_create_and_confirm_intent(
        state,
        req_state,
        platform,
        profile,
        request,
        hyperswitch_domain_models::payments::HeaderPayload::default(),
    ))
    .await?;

    logger::info!(associated_payments_response=?response);

    response
        .get_json_body()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Unexpected response from payments core")
}

#[cfg(feature = "v2")]
pub async fn payment_methods_session_confirm(
    state: SessionState,
    req_state: routes::app::ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    payment_method_session_id: id_type::GlobalPaymentMethodSessionId,
    request: payment_methods::PaymentMethodSessionConfirmRequest,
) -> RouterResponse<payment_methods::PaymentMethodSessionResponse> {
    let db: &dyn StorageInterface = state.store.as_ref();
    request.validate()?;

    // Validate if the session still exists
    let payment_method_session = db
        .get_payment_methods_session(
            platform.get_provider().get_key_store(),
            &payment_method_session_id,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to retrieve payment methods session from db")?;

    let payment_method_session_billing = payment_method_session
        .billing
        .clone()
        .map(|billing| billing.into_inner())
        .map(From::from);

    // Unify the billing address that we receive from the session and from the confirm request
    let unified_billing_address = request
        .payment_method_data
        .billing
        .clone()
        .map(|payment_method_billing| {
            payment_method_billing.unify_address(payment_method_session_billing.as_ref())
        })
        .or_else(|| payment_method_session_billing.clone());

    let storage_type = payment_method_session.storage_type;

    let create_payment_method_request = get_payment_method_create_request(
        request
            .payment_method_data
            .payment_method_data
            .as_ref()
            .get_required_value("payment_method_data")?,
        request.payment_method_type,
        request.payment_method_subtype,
        payment_method_session.customer_id.clone(),
        unified_billing_address.as_ref(),
        Some(&payment_method_session),
        storage_type,
    )
    .attach_printable("Failed to create payment method request")?;

    let (payment_method_response, payment_method) = Box::pin(create_payment_method_core(
        &state,
        &req_state,
        create_payment_method_request.clone(),
        &platform,
        &profile,
    ))
    .await?;

    let parent_payment_method_token = generate_id(consts::ID_LENGTH, "token");

    let token_data = get_pm_list_token_data(
        request.payment_method_type,
        &payment_method,
        create_payment_method_request.storage_type,
    )?;

    // insert the token data into redis
    if let Some(token_data) = token_data {
        let intent_fulfillment_time = common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME;
        pm_routes::ParentPaymentMethodToken::create_key_for_token(&parent_payment_method_token)
            .insert(intent_fulfillment_time, token_data, &state)
            .await?;
    };

    let associated_payment_methods = common_types::payment_methods::AssociatedPaymentMethods {
        payment_method_token: common_types::payment_methods::AssociatedPaymentMethodTokenType::PaymentMethodSessionToken(parent_payment_method_token),
    };

    let update_payment_method_session = hyperswitch_domain_models::payment_methods::PaymentMethodsSessionUpdateEnum::UpdateAssociatedPaymentMethods {
        associated_payment_methods:  Some(vec![associated_payment_methods])
    };

    let payment_method_session = db
        .update_payment_method_session(
            platform.get_provider().get_key_store(),
            &payment_method_session_id,
            update_payment_method_session,
            payment_method_session,
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
            message: "payment methods session does not exist or has expired".to_string(),
        })
        .attach_printable("Failed to update payment methods session from db")?;

    let payments_response = match &payment_method_session.psp_tokenization {
        Some(common_types::payment_methods::PspTokenization {
            tokenization_type: common_enums::TokenizationType::MultiUse,
            ..
        }) => {
            let zero_auth_request = construct_zero_auth_payments_request(
                &request,
                &payment_method_session,
                &payment_method_response,
                payment_method.get_payment_method_subtype(),
            )?;
            let payments_response = Box::pin(create_zero_auth_payment(
                state.clone(),
                req_state,
                platform.clone(),
                profile.clone(),
                zero_auth_request,
            ))
            .await?;

            Some(payments_response)
        }
        Some(common_types::payment_methods::PspTokenization {
            tokenization_type: common_enums::TokenizationType::SingleUse,
            ..
        }) => {
            Box::pin(create_single_use_tokenization_flow(
                state.clone(),
                req_state.clone(),
                platform.clone(),
                profile.clone(),
                &create_payment_method_request.clone(),
                &payment_method_response,
                &payment_method_session,
            ))
            .await?;
            None
        }
        None => None,
    };

    let tokenization_response = match (
        payment_method_session.tokenization_data.clone(),
        payment_method_session.customer_id.clone(),
    ) {
        (Some(tokenization_data), Some(customer_id)) => {
            let tokenization_response = tokenization_core::create_vault_token_core(
                state.clone(),
                platform.get_provider().clone(),
                api_models::tokenization::GenericTokenizationRequest {
                    customer_id: customer_id.clone(),
                    token_request: tokenization_data,
                },
            )
            .await?;
            let token = match tokenization_response {
                services::ApplicationResponse::Json(response) => Some(response),
                _ => None,
            };
            Some(token)
        }
        _ => None,
    };

    logger::debug!(?tokenization_response, "Tokenization response");

    //TODO: update the payment method session with the payment id and payment method id
    let payment_method_session_response = transformers::generate_payment_method_session_response(
        payment_method_session,
        Secret::new("CLIENT_SECRET_REDACTED".to_string()),
        None, // sdk_authorization is not returned for non-create flows
        payments_response,
        (tokenization_response.flatten()),
        payment_method_response.storage_type,
        payment_method_response.card_cvc_token_storage,
        None,
    );

    Ok(services::ApplicationResponse::Json(
        payment_method_session_response,
    ))
}

#[cfg(feature = "v2")]
impl pm_types::SavedPMLPaymentsInfo {
    pub async fn form_payments_info(
        payment_intent: PaymentIntent,
        platform: &domain::Platform,
        profile: domain::Profile,
        db: &dyn StorageInterface,
        key_manager_state: &util_types::keymanager::KeyManagerState,
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
        pma: &api::CustomerPaymentMethodResponseItem,
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

        pm_routes::ParentPaymentMethodToken::create_key_for_token(token)
            .insert(intent_fulfillment_time, token_data, state)
            .await?;

        Ok(())
    }
}

#[cfg(feature = "v2")]
#[allow(clippy::too_many_arguments)]
async fn create_single_use_tokenization_flow(
    state: SessionState,
    req_state: routes::app::ReqState,
    platform: domain::Platform,
    profile: domain::Profile,
    payment_method_create_request: &payment_methods::PaymentMethodCreate,
    payment_method: &api::PaymentMethodResponse,
    payment_method_session: &domain::payment_methods::PaymentMethodSession,
) -> RouterResult<()> {
    let customer_id = payment_method_create_request.customer_id.to_owned();
    let connector_id = payment_method_create_request
        .get_tokenize_connector_id()
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "psp_tokenization.connector_id",
        })
        .attach_printable("Failed to get tokenize connector id")?;

    let db = &state.store;

    let merchant_connector_account_details = db
        .find_merchant_connector_account_by_id(
            &connector_id,
            platform.get_processor().get_key_store(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: connector_id.get_string_repr().to_owned(),
        })
        .attach_printable("error while fetching merchant_connector_account from connector_id")?;
    let auth_type = merchant_connector_account_details
        .get_connector_account_details()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed while parsing value for ConnectorAuthType")?;

    let payment_method_data_request = types::PaymentMethodTokenizationData {
        payment_method_data: domain::PaymentMethodData::try_from(
            payment_method_create_request.payment_method_data.clone(),
        )
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "card_cvc",
        })
        .attach_printable(
            "Failed to convert type from Payment Method Create Data to Payment Method Data",
        )?,
        browser_info: None,
        currency: api_models::enums::Currency::default(),
        amount: None,
        split_payments: None,
        mandate_id: None,
        setup_future_usage: None,
        customer_acceptance: None,
        setup_mandate_details: None,
        payment_method_type: None,
        capture_method: None,
        router_return_url: None,
    };

    let payment_method_session_address = types::PaymentAddress::new(
        None,
        payment_method_session
            .billing
            .clone()
            .map(|address| address.into_inner()),
        None,
        None,
    );

    let mut router_data =
        types::RouterData::<api::PaymentMethodToken, _, types::PaymentsResponseData> {
            flow: std::marker::PhantomData,
            merchant_id: platform.get_provider().get_account().get_id().clone(),
            customer_id: None,
            connector_customer: None,
            connector: merchant_connector_account_details
                .connector_name
                .to_string(),
            payment_id: consts::IRRELEVANT_PAYMENT_INTENT_ID.to_string(), //Static
            attempt_id: consts::IRRELEVANT_PAYMENT_ATTEMPT_ID.to_string(), //Static
            tenant_id: state.tenant.tenant_id.clone(),
            status: common_enums::enums::AttemptStatus::default(),
            payment_method: common_enums::enums::PaymentMethod::Card,
            payment_method_type: None,
            connector_auth_type: auth_type,
            description: None,
            address: payment_method_session_address,
            auth_type: common_enums::enums::AuthenticationType::default(),
            connector_meta_data: None,
            connector_wallets_details: None,
            amount_captured: None,
            access_token: None,
            session_token: None,
            reference_id: None,
            payment_method_token: None,
            recurring_mandate_payment_data: None,
            preprocessing_id: None,
            payment_method_balance: None,
            connector_api_version: None,
            request: payment_method_data_request.clone(),
            response: Err(hyperswitch_domain_models::router_data::ErrorResponse::default()),
            connector_request_reference_id: payment_method_session.id.get_string_repr().to_string(),
            #[cfg(feature = "payouts")]
            payout_method_data: None,
            #[cfg(feature = "payouts")]
            quote_id: None,
            test_mode: None,
            connector_http_status_code: None,
            external_latency: None,
            apple_pay_flow: None,
            frm_metadata: None,
            dispute_id: None,
            refund_id: None,
            payout_id: None,
            connector_response: None,
            payment_method_status: None,
            minor_amount_captured: None,
            integrity_check: Ok(()),
            additional_merchant_data: None,
            header_payload: None,
            connector_mandate_request_reference_id: None,
            authentication_id: None,
            psd2_sca_exemption_type: None,
            raw_connector_response: None,
            is_payment_id_from_merchant: None,
            l2_l3_data: None,
            minor_amount_capturable: None,
            authorized_amount: None,
            customer_document_details: None,
        };

    let payment_method_token_response = Box::pin(tokenization::add_token_for_payment_method(
        &mut router_data,
        payment_method_data_request.clone(),
        state.clone(),
        &merchant_connector_account_details.clone(),
    ))
    .await?;

    let token_response = payment_method_token_response.token.map_err(|err| {
        errors::ApiErrorResponse::ExternalConnectorError {
            code: err.code,
            message: err.message,
            connector: (merchant_connector_account_details.clone())
                .connector_name
                .to_string(),
            status_code: err.status_code,
            reason: err.reason,
        }
    })?;

    let value = domain::SingleUsePaymentMethodToken::get_single_use_token_from_payment_method_token(
        token_response.clone().into(),
        connector_id.clone(),
    );

    let key = domain::SingleUseTokenKey::store_key(&payment_method.id);

    add_single_use_token_to_store(&state, key, value)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to store single use token")?;

    Ok(())
}

#[cfg(feature = "v2")]
async fn add_single_use_token_to_store(
    state: &SessionState,
    key: domain::SingleUseTokenKey,
    value: domain::SingleUsePaymentMethodToken,
) -> CustomResult<(), errors::StorageError> {
    let redis_connection = state
        .store
        .get_redis_conn()
        .map_err(Into::<errors::StorageError>::into)?;

    redis_connection
        .serialize_and_set_key_with_expiry(
            &domain::SingleUseTokenKey::get_store_key(&key).into(),
            value,
            consts::DEFAULT_PAYMENT_METHOD_STORE_TTL,
        )
        .await
        .change_context(errors::StorageError::KVError)
        .attach_printable("Failed to insert payment method token to redis")?;
    Ok(())
}

#[cfg(feature = "v2")]
async fn get_single_use_token_from_store(
    state: &SessionState,
    key: domain::SingleUseTokenKey,
) -> CustomResult<Option<domain::SingleUsePaymentMethodToken>, errors::StorageError> {
    let redis_connection = state
        .store
        .get_redis_conn()
        .map_err(Into::<errors::StorageError>::into)?;

    redis_connection
        .get_and_deserialize_key::<Option<domain::SingleUsePaymentMethodToken>>(
            &domain::SingleUseTokenKey::get_store_key(&key).into(),
            "SingleUsePaymentMethodToken",
        )
        .await
        .change_context(errors::StorageError::KVError)
        .attach_printable("Failed to get payment method token from redis")
}

#[cfg(feature = "v2")]
#[instrument(skip_all)]
async fn fetch_payment_method(
    state: &SessionState,
    provider: &domain::Provider,
    payment_method_id: &id_type::GlobalPaymentMethodId,
) -> RouterResult<domain::PaymentMethod> {
    let db = &state.store;
    let merchant_account = provider.get_account();
    let key_store = provider.get_key_store();

    db.find_payment_method(
        key_store,
        payment_method_id,
        merchant_account.storage_scheme,
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)
    .attach_printable("Payment method not found for network token status check")
}

#[cfg(feature = "v2")]
pub async fn check_network_token_status(
    state: SessionState,
    provider: domain::Provider,
    payment_method_id: id_type::GlobalPaymentMethodId,
) -> RouterResponse<payment_methods::NetworkTokenStatusCheckResponse> {
    // Retrieve the payment method from the database
    let payment_method = fetch_payment_method(&state, &provider, &payment_method_id).await?;

    // Call the network token status check function
    let network_token_status_check_response = if payment_method.status
        == common_enums::PaymentMethodStatus::Active
    {
        // Check if the payment method has network token data
        when(
            payment_method
                .network_token_requestor_reference_id
                .is_none(),
            || {
                Err(errors::ApiErrorResponse::InvalidDataValue {
                    field_name: "payment_method_id",
                })
            },
        )?;
        match network_tokenization::do_status_check_for_network_token(&state, &payment_method).await
        {
            Ok(network_token_details) => {
                let status = match network_token_details.payload.token_status {
                    pm_types::TokenStatus::Active => api_enums::TokenStatus::Active,
                    pm_types::TokenStatus::Suspended => api_enums::TokenStatus::Suspended,
                    pm_types::TokenStatus::Inactive => api_enums::TokenStatus::Inactive,
                    pm_types::TokenStatus::Expired => api_enums::TokenStatus::Expired,
                    pm_types::TokenStatus::Deleted => api_enums::TokenStatus::Deleted,
                };

                payment_methods::NetworkTokenStatusCheckResponse::SuccessResponse(
                    payment_methods::NetworkTokenStatusCheckSuccessResponse {
                        status,
                        token_expiry_month: network_token_details.payload.token_expiry_month,
                        token_expiry_year: network_token_details.payload.token_expiry_year,
                        card_last_four: network_token_details.payload.card_last_four,
                        card_expiry_month: network_token_details.payload.card_expiry_month,
                        card_expiry_year: network_token_details.payload.card_expiry_year,
                        token_last_four: network_token_details.payload.token_last_four,
                        payment_method_id,
                        customer_id: payment_method
                            .customer_id
                            .clone()
                            .get_required_value("GlobalCustomerId")?,
                    },
                )
            }
            Err(e) => {
                let err_message = e.current_context().to_string();
                logger::debug!("Network token status check failed: {:?}", e);

                payment_methods::NetworkTokenStatusCheckResponse::FailureResponse(
                    payment_methods::NetworkTokenStatusCheckFailureResponse {
                        error_message: err_message,
                    },
                )
            }
        }
    } else {
        let err_message = "Payment Method is not active".to_string();
        logger::debug!("Payment Method is not active");

        payment_methods::NetworkTokenStatusCheckResponse::FailureResponse(
            payment_methods::NetworkTokenStatusCheckFailureResponse {
                error_message: err_message,
            },
        )
    };
    Ok(services::ApplicationResponse::Json(
        network_token_status_check_response,
    ))
}

#[cfg(feature = "v2")]
pub async fn payment_method_get_token_details_core(
    state: SessionState,
    provider: domain::Provider,
    temporary_token: String,
) -> RouterResponse<payment_methods::PaymentMethodGetTokenDetailsResponse> {
    let token_data = pm_routes::ParentPaymentMethodToken::create_key_for_token(&temporary_token)
        .get_data_for_token(&state)
        .await;

    match token_data {
        Ok(storage::PaymentTokenData::PermanentCard(card_token_data)) => {
            Ok(services::ApplicationResponse::Json(
                payment_methods::PaymentMethodGetTokenDetailsResponse {
                    id: card_token_data.payment_method_id,
                    payment_method_token: temporary_token,
                },
            ))
        }
        Ok(_) => Err(errors::ApiErrorResponse::PaymentMethodNotFound.into()),
        Err(e) => Err(e),
    }
}

#[cfg(feature = "v2")]
impl<'a> pm_types::PaymentMethodUpdateHandler<'a> {
    pub async fn generate(
        state: &'a SessionState,
        platform: &'a domain::Platform,
        profile: &'a domain::Profile,
        request: DomainPaymentMethodUpdate,
        payment_method_id: &'a id_type::GlobalPaymentMethodId,
    ) -> RouterResult<Self> {
        let payment_method = state
            .store
            .find_payment_method(
                platform.get_provider().get_key_store(),
                payment_method_id,
                platform.get_provider().get_account().storage_scheme,
            )
            .await
            .to_not_found_response(errors::ApiErrorResponse::PaymentMethodNotFound)?;

        let handler = Self {
            platform,
            profile,
            request,
            payment_method,
            state,
        };
        handler.validate()?;
        Ok(handler)
    }

    fn validate(&self) -> RouterResult<()> {
        let payment_method = &self.payment_method;
        when(
            payment_method.status == enums::PaymentMethodStatus::Inactive,
            || {
                Err(errors::ApiErrorResponse::InvalidRequestData {
                    message: "Inactive Payment Method cannot be updated".to_string(),
                })
            },
        )?;

        Ok(())
    }

    fn is_card_metadata_changed_for_same_fingerprint(
        &self,
        vault_request_data: &domain::PaymentMethodVaultingData,
    ) -> bool {
        let updated_card_metadata = vault_request_data
            .get_payment_methods_data()
            .get_card_details()
            .map(|card_details| (card_details.card_holder_name, card_details.nick_name));

        let existing_card_metadata = self
            .payment_method
            .payment_method_data
            .clone()
            .and_then(|payment_method_data| payment_method_data.into_inner().get_card_details())
            .map(|card_details| (card_details.card_holder_name, card_details.nick_name));

        match (existing_card_metadata, updated_card_metadata) {
            (Some(existing), Some(updated)) => existing != updated,
            (None, Some(_)) => true,
            _ => false,
        }
    }

    pub async fn update_cvc_if_required(
        &self,
    ) -> RouterResult<Option<payment_methods::CardCVCTokenStorageDetails>> {
        let card_cvc = self.request.fetch_card_cvc_update();
        let card_cvc_token_details = card_cvc
            .async_map(|cvc| {
                vault::insert_cvc_using_payment_token(
                    self.state,
                    self.payment_method.get_id(),
                    cvc,
                    common_utils::consts::DEFAULT_INTENT_FULFILLMENT_TIME,
                    self.platform.get_provider().get_key_store(),
                )
            })
            .await
            .transpose()
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to insert encrypted cvc in redis")?;

        Ok(card_cvc_token_details)
    }

    pub async fn perform_vaulting_operations_if_required(
        &self,
    ) -> RouterResult<(
        Option<domain::PaymentMethodVaultingData>,
        Option<pm_types::AddVaultResponse>,
    )> {
        if !self.request.is_payment_method_metadata_update() {
            return Ok((None, None));
        }

        let pmd: domain::PaymentMethodVaultingData = vault::retrieve_payment_method_from_vault(
            self.state,
            self.platform,
            self.profile,
            &self.payment_method,
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve payment method from vault")?
        .data;

        let vault_request_data = self
            .request
            .payment_method_data
            .clone()
            .map(|payment_method_data| {
                pm_transforms::generate_pm_vaulting_req_from_update_request(
                    pmd,
                    payment_method_data,
                )
            })
            .ok_or(errors::ApiErrorResponse::MissingRequiredField {
                field_name: "payment_method_data",
            })?;

        let current_vault_id = self.payment_method.locker_id.clone();

        let fingerprint_id_from_vault = vault::get_fingerprint_id_for_payment_method(
            self.state,
            &vault_request_data,
            self.payment_method
                .customer_id
                .clone()
                .get_required_value("GlobalCustomerId")?
                .get_string_repr()
                .to_owned(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to get fingerprint_id from vault")?;

        let is_stored_payment_method_same_as_in_request = self
            .payment_method
            .locker_fingerprint_id
            .clone()
            .map(|data| data == fingerprint_id_from_vault);

        match is_stored_payment_method_same_as_in_request {
            Some(true) => {
                if self.is_card_metadata_changed_for_same_fingerprint(&vault_request_data) {
                    logger::info!(
                        "Payment method fingerprint is same, updating only payment method metadata in db"
                    );
                    Ok((Some(vault_request_data), None))
                } else {
                    logger::info!(
                        "Payment method vault data is same as in request, skipping vault update"
                    );
                    Ok((None, None))
                }
            }
            Some(false) | None => {
                logger::info!("Payment method vault data is different from request, proceeding with vault update");
                let (vault_response, _) = vault_payment_method(
                    self.state,
                    &vault_request_data,
                    self.platform,
                    self.profile,
                    current_vault_id,
                    fingerprint_id_from_vault,
                    &self
                        .payment_method
                        .customer_id
                        .to_owned()
                        .get_required_value("GlobalCustomerId")?,
                )
                .await
                .attach_printable("Failed to add payment method in vault")?;

                Ok((Some(vault_request_data), Some(vault_response)))
            }
        }
    }

    pub async fn update_payment_method_if_required(
        &mut self,
        vault_request_data: Option<domain::PaymentMethodVaultingData>,
        vault_resp: Option<pm_types::AddVaultResponse>,
        network_tokenization_resp: Option<NetworkTokenPaymentMethodDetails>,
    ) -> RouterResult<()> {
        if !self.request.is_payment_method_update_required() && network_tokenization_resp.is_none()
        {
            return Ok(());
        }

        let pm_status = match self.request.acknowledgement_status {
            Some(common_enums::AcknowledgementStatus::Authenticated) => {
                Some(common_enums::PaymentMethodStatus::Active)
            }
            Some(common_enums::AcknowledgementStatus::Failed) => {
                Some(common_enums::PaymentMethodStatus::Inactive)
            }
            None => None,
        };

        let db = self.state.store.as_ref();

        let pm_update = create_pm_additional_data_update(
            vault_request_data.as_ref(),
            self.state,
            self.platform.get_provider().get_key_store(),
            vault_resp
                .as_ref()
                .map(|resp| resp.vault_id.get_string_repr().to_owned()),
            vault_resp
                .as_ref()
                .and_then(|resp| resp.fingerprint_id.clone()),
            &self.payment_method,
            self.request.connector_token_details.clone(),
            self.request.network_transaction_id.clone(),
            network_tokenization_resp,
            None,
            None,
            None,
            pm_status,
            self.platform.get_initiator(),
        )
        .await
        .attach_printable("Unable to create Payment method data")?;

        let updated_payment_method = db
            .update_payment_method(
                self.platform.get_provider().get_key_store(),
                self.payment_method.clone(),
                pm_update,
                self.platform.get_provider().get_account().storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to update payment method in db")?;

        self.payment_method = updated_payment_method;
        Ok(())
    }

    pub fn generate_response(
        &self,
        card_cvc_token_details: Option<payment_methods::CardCVCTokenStorageDetails>,
    ) -> RouterResult<payment_methods::PaymentMethodResponse> {
        let response = pm_transforms::generate_payment_method_response(
            &self.payment_method,
            &None,
            common_enums::StorageType::Persistent,
            card_cvc_token_details,
            self.payment_method.customer_id.clone(),
            None,
            self.payment_method
                .payment_method_billing_address
                .clone()
                .map(|billing| billing.get_inner().clone().into()),
            self.request.acknowledgement_status,
        )?;

        Ok(response)
    }
}
