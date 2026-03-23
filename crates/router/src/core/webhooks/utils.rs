use std::marker::PhantomData;

use base64::Engine;
use common_enums::MerchantAccountType;
use common_utils::{
    consts,
    crypto::{self, GenerateDigest},
    errors::CustomResult,
    ext_traits::ValueExt,
    types::CreatedBy,
};
use error_stack::{Report, ResultExt};
use redis_interface as redis;
use router_env::tracing;

use super::MERCHANT_ID;
use crate::{
    core::{
        errors::{self},
        metrics,
        payments::helpers,
    },
    db::{get_and_deserialize_key, StorageInterface},
    errors::RouterResult,
    routes::app::SessionStateInfo,
    services::logger,
    types::{self, api, domain, PaymentAddress},
    SessionState,
};

const IRRELEVANT_ATTEMPT_ID_IN_SOURCE_VERIFICATION_FLOW: &str =
    "irrelevant_attempt_id_in_source_verification_flow";
const IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_SOURCE_VERIFICATION_FLOW: &str =
    "irrelevant_connector_request_reference_id_in_source_verification_flow";

/// Check whether the merchant has configured to disable the webhook `event` for the `connector`
/// First check for the key "whconf_{merchant_id}_{connector_id}" in redis,
/// if not found, fetch from configs table in database
pub async fn is_webhook_event_disabled(
    db: &dyn StorageInterface,
    connector_id: &str,
    merchant_id: &common_utils::id_type::MerchantId,
    event: &api::IncomingWebhookEvent,
) -> bool {
    let redis_key = merchant_id.get_webhook_config_disabled_events_key(connector_id);
    let merchant_webhook_disable_config_result: CustomResult<
        api::MerchantWebhookConfig,
        redis_interface::errors::RedisError,
    > = get_and_deserialize_key(db, &redis_key, "MerchantWebhookConfig").await;

    match merchant_webhook_disable_config_result {
        Ok(merchant_webhook_config) => merchant_webhook_config.contains(event),
        Err(..) => {
            //if failed to fetch from redis. fetch from db and populate redis
            db.find_config_by_key(&redis_key)
                .await
                .map(|config| {
                    match serde_json::from_str::<api::MerchantWebhookConfig>(&config.config) {
                        Ok(set) => set.contains(event),
                        Err(err) => {
                            logger::warn!(?err, "error while parsing merchant webhook config");
                            false
                        }
                    }
                })
                .unwrap_or_else(|err| {
                    logger::warn!(?err, "error while fetching merchant webhook config");
                    false
                })
        }
    }
}

pub async fn construct_webhook_router_data(
    state: &SessionState,
    connector_name: &str,
    merchant_connector_account: domain::MerchantConnectorAccount,
    platform: &domain::Platform,
    connector_wh_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    request_details: &api::IncomingWebhookRequestDetails<'_>,
) -> CustomResult<types::VerifyWebhookSourceRouterData, errors::ApiErrorResponse> {
    let auth_type: types::ConnectorAuthType =
        helpers::MerchantConnectorAccountType::DbVal(Box::new(merchant_connector_account.clone()))
            .get_connector_account_details()
            .parse_value("ConnectorAuthType")
            .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let router_data = types::RouterData {
        flow: PhantomData,
        merchant_id: platform.get_processor().get_account().get_id().clone(),
        connector: connector_name.to_string(),
        customer_id: None,
        tenant_id: state.tenant.tenant_id.clone(),
        payment_id: common_utils::id_type::PaymentId::get_irrelevant_id("source_verification_flow")
            .get_string_repr()
            .to_owned(),
        attempt_id: IRRELEVANT_ATTEMPT_ID_IN_SOURCE_VERIFICATION_FLOW.to_string(),
        status: diesel_models::enums::AttemptStatus::default(),
        payment_method: diesel_models::enums::PaymentMethod::default(),
        payment_method_type: None,
        connector_auth_type: auth_type,
        description: None,
        address: PaymentAddress::default(),
        auth_type: diesel_models::enums::AuthenticationType::default(),
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        minor_amount_captured: None,
        request: types::VerifyWebhookSourceRequestData {
            webhook_headers: request_details.headers.clone(),
            webhook_body: request_details.body.to_vec().clone(),
            merchant_secret: connector_wh_secrets.to_owned(),
            webhook_uri: request_details.uri.clone(),
        },
        response: Err(types::ErrorResponse::default()),
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        connector_customer: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        connector_request_reference_id:
            IRRELEVANT_CONNECTOR_REQUEST_REFERENCE_ID_IN_SOURCE_VERIFICATION_FLOW.to_string(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        payment_method_balance: None,
        payment_method_status: None,
        connector_api_version: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        refund_id: None,
        dispute_id: None,
        payout_id: None,
        connector_response: None,
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
    Ok(router_data)
}

#[inline]
pub(crate) fn get_idempotent_event_id(
    primary_object_id: &str,
    event_type: types::storage::enums::EventType,
    delivery_attempt: types::storage::enums::WebhookDeliveryAttempt,
) -> Result<String, Report<errors::WebhooksFlowError>> {
    use crate::types::storage::enums::WebhookDeliveryAttempt;

    const EVENT_ID_SUFFIX_LENGTH: usize = 8;

    let common_prefix = format!("{primary_object_id}_{event_type}");

    // Hash the common prefix with SHA256 and encode with URL-safe base64 without padding
    let digest = crypto::Sha256
        .generate_digest(common_prefix.as_bytes())
        .change_context(errors::WebhooksFlowError::IdGenerationFailed)
        .attach_printable("Failed to generate idempotent event ID")?;
    let base_encoded = consts::BASE64_ENGINE_URL_SAFE_NO_PAD.encode(digest);

    let result = match delivery_attempt {
        WebhookDeliveryAttempt::InitialAttempt => base_encoded,
        WebhookDeliveryAttempt::AutomaticRetry | WebhookDeliveryAttempt::ManualRetry => {
            common_utils::generate_id(EVENT_ID_SUFFIX_LENGTH, &base_encoded)
        }
    };

    Ok(result)
}

#[inline]
pub(crate) fn generate_event_id() -> String {
    common_utils::generate_time_ordered_id("evt")
}

pub fn increment_webhook_outgoing_received_count(merchant_id: &common_utils::id_type::MerchantId) {
    metrics::WEBHOOK_OUTGOING_RECEIVED_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, merchant_id.clone())),
    )
}

pub fn increment_webhook_outgoing_not_received_count(
    merchant_id: &common_utils::id_type::MerchantId,
) {
    metrics::WEBHOOK_OUTGOING_NOT_RECEIVED_COUNT.add(
        1,
        router_env::metric_attributes!((MERCHANT_ID, merchant_id.clone())),
    );
}

pub fn is_outgoing_webhook_disabled(
    state: &SessionState,
    webhook_url_result: &Result<String, Report<errors::WebhooksFlowError>>,
    business_profile: &domain::Profile,
    idempotent_event_id: &str,
) -> bool {
    if !state.conf.webhooks.outgoing_enabled
        || webhook_url_result.is_err()
        || webhook_url_result.as_ref().is_ok_and(String::is_empty)
    {
        logger::debug!(
            business_profile_id=?business_profile.get_id(),
            %idempotent_event_id,
            "Outgoing webhooks are disabled in application configuration, or merchant webhook URL \
             could not be obtained; skipping outgoing webhooks for event"
        );
        return true;
    }
    false
}

const WEBHOOK_LOCK_PREFIX: &str = "WEBHOOK_LOCK";

pub(super) async fn perform_redis_lock<A>(
    state: &A,
    unique_locking_key: &str,
    merchant_id: common_utils::id_type::MerchantId,
) -> RouterResult<Option<String>>
where
    A: SessionStateInfo,
{
    let lock_value: String = uuid::Uuid::new_v4().to_string();
    let redis_conn = state
        .store()
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error connecting to redis")?;

    let redis_locking_key = format!(
        "{}_{}_{}",
        WEBHOOK_LOCK_PREFIX,
        merchant_id.get_string_repr(),
        unique_locking_key
    );
    let redis_lock_expiry_seconds = state.conf().webhooks.redis_lock_expiry_seconds;

    let redis_lock_result = redis_conn
        .set_key_if_not_exists_with_expiry(
            &redis_locking_key.as_str().into(),
            lock_value.clone(),
            Some(i64::from(redis_lock_expiry_seconds)),
        )
        .await;

    match redis_lock_result {
        Ok(redis::SetnxReply::KeySet) => {
            logger::info!("Lock acquired for for {redis_locking_key}");
            Ok(Some(lock_value))
        }
        Ok(redis::SetnxReply::KeyNotSet) => {
            logger::info!("Lock already held for {redis_locking_key}");
            Ok(None)
        }
        Err(err) => Err(err
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error acquiring redis lock")),
    }
}

/// Fetches the default business profile for the provider merchant.
#[cfg(feature = "v1")]
async fn fetch_provider_profile(
    state: &SessionState,
    platform: &domain::Platform,
) -> RouterResult<domain::Profile> {
    let profile_id = platform
        .get_provider()
        .get_account()
        .get_default_profile()
        .as_ref()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Platform provider merchant has no default profile configured")?;
    state
        .store
        .find_business_profile_by_profile_id(platform.get_provider().get_key_store(), profile_id)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to fetch provider merchant default business profile")
}

/// Fetches the default business profile for the provider merchant.
#[cfg(feature = "v2")]
async fn fetch_provider_profile(
    state: &SessionState,
    platform: &domain::Platform,
) -> RouterResult<domain::Profile> {
    state
        .store
        .list_profile_by_merchant_id(
            platform.get_provider().get_key_store(),
            platform.get_provider().get_account().get_id(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to list provider merchant profiles")?
        .into_iter()
        .next()
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Platform provider merchant has no profiles configured")
}

/// Resolves the outgoing webhook recipient for **direct payment flows** where
/// `platform.get_initiator()` is available at call time.
///
/// Returns `(business_profile, compatible_connector)` for the recipient:
/// - If the platform merchant initiated → uses the provider's default profile and
///   compatible connector.
/// - Otherwise → use the processor's business profile
///   and the compatible connector.
pub(crate) async fn resolve_webhook_recipient_from_initiator(
    state: &SessionState,
    platform: &domain::Platform,
    processor_profile: domain::Profile,
) -> RouterResult<(domain::Profile, Option<api_models::enums::Connector>)> {
    match platform.get_initiator() {
        Some(domain::Initiator::Api {
            merchant_account_type: MerchantAccountType::Platform,
            ..
        }) => {
            let profile = fetch_provider_profile(state, platform).await?;
            Ok((
                profile,
                platform
                    .get_provider()
                    .get_account()
                    .get_compatible_connector(),
            ))
        }
        Some(domain::Initiator::Api {
            merchant_account_type: MerchantAccountType::Connected,
            ..
        })
        | Some(domain::Initiator::Api {
            merchant_account_type: MerchantAccountType::Standard,
            ..
        })
        | Some(domain::Initiator::Jwt { .. })
        | Some(domain::Initiator::EmbeddedToken { .. })
        | Some(domain::Initiator::Admin)
        | None => Ok((
            processor_profile,
            platform
                .get_processor()
                .get_account()
                .get_compatible_connector(),
        )),
    }
}

/// Resolves the outgoing webhook recipient for the **incoming webhook flow** where the initiator
/// must be inferred from the `created_by` field stored on the payment object.
///
/// Returns `(business_profile, compatible_connector)` for the recipient:
/// - If `created_by` holds a merchant ID that exactly matches the platform **provider** merchant
///   ID → fetches the provider's default business profile and routes the webhook there.
/// - Otherwise (JWT, absent `created_by`, same merchant as processor, or any unrecognised ID)
///   → falls back to the processor's business profile and compatible connector.
pub(crate) async fn resolve_webhook_recipient_from_created_by(
    state: &SessionState,
    platform: &domain::Platform,
    processor_profile: domain::Profile,
    created_by: Option<&CreatedBy>,
) -> RouterResult<(domain::Profile, Option<api_models::enums::Connector>)> {
    let is_provider_initiated = created_by
        .map(|c| c.is_provider_initiated(platform.get_provider().get_account().get_id()))
        .unwrap_or_default();

    if is_provider_initiated {
        let profile = fetch_provider_profile(state, platform).await?;
        Ok((
            profile,
            platform
                .get_provider()
                .get_account()
                .get_compatible_connector(),
        ))
    } else {
        Ok((
            processor_profile,
            platform
                .get_processor()
                .get_account()
                .get_compatible_connector(),
        ))
    }
}

pub(super) async fn free_redis_lock<A>(
    state: &A,
    unique_locking_key: &str,
    merchant_id: common_utils::id_type::MerchantId,
    lock_value: Option<String>,
) -> RouterResult<()>
where
    A: SessionStateInfo,
{
    let redis_conn = state
        .store()
        .get_redis_conn()
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Error connecting to redis")?;

    let redis_locking_key = format!(
        "{}_{}_{}",
        WEBHOOK_LOCK_PREFIX,
        merchant_id.get_string_repr(),
        unique_locking_key
    );
    match redis_conn
        .get_key::<Option<String>>(&redis_locking_key.as_str().into())
        .await
    {
        Ok(val) => {
            if val == lock_value {
                match redis_conn
                    .delete_key(&redis_locking_key.as_str().into())
                    .await
                {
                    Ok(redis::types::DelReply::KeyDeleted) => {
                        logger::info!("Lock freed {redis_locking_key}");
                        tracing::Span::current().record("redis_lock_released", redis_locking_key);
                        Ok(())
                    }
                    Ok(redis::types::DelReply::KeyNotDeleted) => Err(
                        errors::ApiErrorResponse::InternalServerError,
                    )
                    .attach_printable("Status release lock called but key is not found in redis"),
                    Err(error) => Err(error)
                        .change_context(errors::ApiErrorResponse::InternalServerError)
                        .attach_printable("Error while deleting redis key"),
                }
            } else {
                Err(errors::ApiErrorResponse::InternalServerError)
                            .attach_printable("The redis value which acquired the lock is not equal to the redis value requesting for releasing the lock")
            }
        }
        Err(error) => Err(error)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error while deleting redis key"),
    }
}
