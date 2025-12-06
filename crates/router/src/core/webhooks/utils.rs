use std::marker::PhantomData;

use base64::Engine;
use common_utils::{
    consts,
    crypto::{self, GenerateDigest},
    errors::CustomResult,
    ext_traits::ValueExt,
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
