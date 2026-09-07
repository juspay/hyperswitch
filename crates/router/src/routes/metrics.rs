pub mod bg_metrics_collector;
pub mod request;

use std::time::Duration;

use router_env::{counter_metric, global_meter, histogram_metric_f64, metric_attributes};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(HEALTH_METRIC, GLOBAL_METER); // No. of health API hits
counter_metric!(KV_MISS, GLOBAL_METER); // No. of KV misses

// API Level Metrics
counter_metric!(REQUESTS_RECEIVED, GLOBAL_METER);
histogram_metric_f64!(REQUEST_TIME, GLOBAL_METER);

histogram_metric_f64!(
    PAYMENT_OPERATION_DURATION,
    GLOBAL_METER,
    name: "payment.operation.duration",
    description: "Duration of completed payment domain operations",
    unit: "s",
);
histogram_metric_f64!(
    MICROSERVICE_CLIENT_CALL_DURATION,
    GLOBAL_METER,
    name: "microservice.client.call.duration",
    description: "Duration of completed internal microservice call attempts",
    unit: "s",
);
histogram_metric_f64!(
    VAULT_CALL_DURATION,
    GLOBAL_METER,
    name: "vault.call.duration",
    description: "Duration of completed legacy vault call attempts",
    unit: "s",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum MerchantMode {
    Modular,
    NonModular,
}

impl MerchantMode {
    pub const fn from_modular_enabled(enabled: bool) -> Self {
        if enabled {
            Self::Modular
        } else {
            Self::NonModular
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaymentMetricsContext {
    pub merchant_mode: MerchantMode,
}

impl PaymentMetricsContext {
    pub const fn payments_confirm(merchant_mode: MerchantMode) -> Self {
        Self { merchant_mode }
    }
}

pub fn record_payment_confirm<T, E>(
    result: &Result<T, E>,
    duration: Duration,
    context: PaymentMetricsContext,
) {
    let outcome = if result.is_ok() { "success" } else { "failure" };
    let merchant_mode: &'static str = context.merchant_mode.into();
    let attributes = metric_attributes!(
        ("operation", "confirm"),
        ("merchant_mode", merchant_mode),
        ("outcome", outcome),
    );

    PAYMENT_OPERATION_DURATION.record(duration.as_secs_f64(), attributes);
}

pub fn record_microservice_call<T, E>(
    result: &Result<T, E>,
    duration: Duration,
    service: &'static str,
    operation: &'static str,
    context: PaymentMetricsContext,
) {
    let merchant_mode: &'static str = context.merchant_mode.into();
    MICROSERVICE_CLIENT_CALL_DURATION.record(
        duration.as_secs_f64(),
        metric_attributes!(
            ("service", service),
            ("operation", operation),
            ("merchant_mode", merchant_mode),
            (
                "outcome",
                if result.is_ok() { "success" } else { "failure" }
            ),
        ),
    );
}

pub fn record_vault_call(
    duration: Duration,
    operation: &'static str,
    succeeded: bool,
    context: PaymentMetricsContext,
) {
    let merchant_mode: &'static str = context.merchant_mode.into();
    VAULT_CALL_DURATION.record(
        duration.as_secs_f64(),
        metric_attributes!(
            ("operation", operation),
            ("merchant_mode", merchant_mode),
            ("outcome", if succeeded { "success" } else { "failure" }),
        ),
    );
}

// Operation Level Metrics
counter_metric!(PAYMENT_OPS_COUNT, GLOBAL_METER);

counter_metric!(PAYMENT_COUNT, GLOBAL_METER);
counter_metric!(SUCCESSFUL_PAYMENT, GLOBAL_METER);
//TODO: This can be removed, added for payment list debugging
histogram_metric_f64!(PAYMENT_LIST_LATENCY, GLOBAL_METER);

histogram_metric_f64!(PAYMENT_LIST_OPENSEARCH_LATENCY, GLOBAL_METER);

counter_metric!(REFUND_COUNT, GLOBAL_METER);
counter_metric!(SUCCESSFUL_REFUND, GLOBAL_METER);

counter_metric!(PAYMENT_CANCEL_COUNT, GLOBAL_METER);
counter_metric!(SUCCESSFUL_CANCEL, GLOBAL_METER);

counter_metric!(PAYMENT_EXTEND_AUTHORIZATION_COUNT, GLOBAL_METER);
counter_metric!(SUCCESSFUL_EXTEND_AUTHORIZATION_COUNT, GLOBAL_METER);

counter_metric!(MANDATE_COUNT, GLOBAL_METER);
counter_metric!(SUBSEQUENT_MANDATE_PAYMENT, GLOBAL_METER);

// Manual retry metrics
counter_metric!(MANUAL_RETRY_REQUEST_COUNT, GLOBAL_METER);
counter_metric!(MANUAL_RETRY_COUNT, GLOBAL_METER);
counter_metric!(MANUAL_RETRY_VALIDATION_FAILED, GLOBAL_METER);
counter_metric!(CVC_READ_ONLY_RETRIEVAL, GLOBAL_METER);
counter_metric!(CVC_DELETED_AFTER_SUCCESS, GLOBAL_METER);
counter_metric!(CVC_DELETION_FAILURE, GLOBAL_METER);
counter_metric!(MANUAL_RETRY_CVC_LOOKUP_MISS, GLOBAL_METER);

counter_metric!(STORED_TO_LOCKER, GLOBAL_METER);
counter_metric!(GET_FROM_LOCKER, GLOBAL_METER);
counter_metric!(DELETE_FROM_LOCKER, GLOBAL_METER);

counter_metric!(CREATED_TOKENIZED_CARD, GLOBAL_METER);
counter_metric!(DELETED_TOKENIZED_CARD, GLOBAL_METER);
counter_metric!(GET_TOKENIZED_CARD, GLOBAL_METER);
counter_metric!(TOKENIZED_DATA_COUNT, GLOBAL_METER); // Tokenized data added
counter_metric!(RETRIED_DELETE_DATA_COUNT, GLOBAL_METER); // Tokenized data retried

counter_metric!(CUSTOMER_CREATED, GLOBAL_METER);
counter_metric!(CUSTOMER_REDACTED, GLOBAL_METER);

counter_metric!(API_KEY_CREATED, GLOBAL_METER);
counter_metric!(API_KEY_REVOKED, GLOBAL_METER);

counter_metric!(MCA_CREATE, GLOBAL_METER);

// Flow Specific Metrics

histogram_metric_f64!(CONNECTOR_REQUEST_TIME, GLOBAL_METER);
counter_metric!(SESSION_TOKEN_CREATED, GLOBAL_METER);

counter_metric!(CONNECTOR_CALL_COUNT, GLOBAL_METER); // Attributes needed

counter_metric!(THREE_DS_PAYMENT_COUNT, GLOBAL_METER);
counter_metric!(THREE_DS_DOWNGRADE_COUNT, GLOBAL_METER);

counter_metric!(RESPONSE_DESERIALIZATION_FAILURE, GLOBAL_METER);
counter_metric!(CONNECTOR_ERROR_RESPONSE_COUNT, GLOBAL_METER);
counter_metric!(REQUEST_TIMEOUT_COUNT, GLOBAL_METER);

counter_metric!(EXECUTE_PRETASK_COUNT, GLOBAL_METER);
counter_metric!(CONNECTOR_PAYMENT_METHOD_TOKENIZATION, GLOBAL_METER);
counter_metric!(PREPROCESSING_STEPS_COUNT, GLOBAL_METER);
counter_metric!(CONNECTOR_CUSTOMER_CREATE, GLOBAL_METER);
counter_metric!(REDIRECTION_TRIGGERED, GLOBAL_METER);

// Connector Level Metric
counter_metric!(REQUEST_BUILD_FAILURE, GLOBAL_METER);
// Connector http status code metrics
counter_metric!(CONNECTOR_HTTP_STATUS_CODE_1XX_COUNT, GLOBAL_METER);
counter_metric!(CONNECTOR_HTTP_STATUS_CODE_2XX_COUNT, GLOBAL_METER);
counter_metric!(CONNECTOR_HTTP_STATUS_CODE_3XX_COUNT, GLOBAL_METER);
counter_metric!(CONNECTOR_HTTP_STATUS_CODE_4XX_COUNT, GLOBAL_METER);
counter_metric!(CONNECTOR_HTTP_STATUS_CODE_5XX_COUNT, GLOBAL_METER);

// Service Level
counter_metric!(CARD_LOCKER_FAILURES, GLOBAL_METER);
counter_metric!(CARD_LOCKER_SUCCESSFUL_RESPONSE, GLOBAL_METER);
counter_metric!(TEMP_LOCKER_FAILURES, GLOBAL_METER);
histogram_metric_f64!(CARD_ADD_TIME, GLOBAL_METER);
histogram_metric_f64!(CARD_GET_TIME, GLOBAL_METER);
histogram_metric_f64!(CARD_DELETE_TIME, GLOBAL_METER);

// Apple Pay Flow Metrics
counter_metric!(APPLE_PAY_MANUAL_FLOW, GLOBAL_METER);
counter_metric!(APPLE_PAY_SIMPLIFIED_FLOW, GLOBAL_METER);
counter_metric!(APPLE_PAY_MANUAL_FLOW_SUCCESSFUL_PAYMENT, GLOBAL_METER);
counter_metric!(APPLE_PAY_SIMPLIFIED_FLOW_SUCCESSFUL_PAYMENT, GLOBAL_METER);
counter_metric!(APPLE_PAY_MANUAL_FLOW_FAILED_PAYMENT, GLOBAL_METER);
counter_metric!(APPLE_PAY_SIMPLIFIED_FLOW_FAILED_PAYMENT, GLOBAL_METER);

// Metrics for Payment Auto Retries
counter_metric!(AUTO_RETRY_ELIGIBLE_REQUEST_COUNT, GLOBAL_METER);
counter_metric!(AUTO_RETRY_GSM_MISS_COUNT, GLOBAL_METER);
counter_metric!(AUTO_RETRY_GSM_FETCH_FAILURE_COUNT, GLOBAL_METER);
counter_metric!(AUTO_RETRY_GSM_MATCH_COUNT, GLOBAL_METER);
counter_metric!(AUTO_RETRY_EXHAUSTED_COUNT, GLOBAL_METER);
counter_metric!(AUTO_RETRY_PAYMENT_COUNT, GLOBAL_METER);

// Metrics for Payout Auto Retries
counter_metric!(AUTO_PAYOUT_RETRY_ELIGIBLE_REQUEST_COUNT, GLOBAL_METER);
counter_metric!(AUTO_PAYOUT_RETRY_GSM_MISS_COUNT, GLOBAL_METER);
counter_metric!(AUTO_PAYOUT_RETRY_GSM_FETCH_FAILURE_COUNT, GLOBAL_METER);
counter_metric!(AUTO_PAYOUT_RETRY_GSM_MATCH_COUNT, GLOBAL_METER);
counter_metric!(AUTO_PAYOUT_RETRY_EXHAUSTED_COUNT, GLOBAL_METER);
counter_metric!(AUTO_RETRY_PAYOUT_COUNT, GLOBAL_METER);

// Scheduler / Process Tracker related metrics
counter_metric!(TASKS_ADDED_COUNT, GLOBAL_METER); // Tasks added to process tracker
counter_metric!(TASK_ADDITION_FAILURES_COUNT, GLOBAL_METER); // Failures in task addition to process tracker
counter_metric!(TASKS_RESET_COUNT, GLOBAL_METER); // Tasks reset in process tracker for requeue flow

counter_metric!(OFFER_ENGINE_LIST_FAILURES, GLOBAL_METER);

// Offer Engine notification (Process Tracker) metrics
counter_metric!(OFFER_ENGINE_NOTIFY_TASKS_SCHEDULED, GLOBAL_METER);
counter_metric!(OFFER_ENGINE_NOTIFY_SCHEDULE_FAILURES, GLOBAL_METER);
counter_metric!(OFFER_ENGINE_NOTIFY_SUCCESS, GLOBAL_METER);
counter_metric!(OFFER_ENGINE_NOTIFY_TERMINAL_FAILURES, GLOBAL_METER);

// Access token metrics
//
// A counter to indicate the number of new access tokens created
counter_metric!(ACCESS_TOKEN_CREATION, GLOBAL_METER);

// A counter to indicate the access token cache hits
counter_metric!(ACCESS_TOKEN_CACHE_HIT, GLOBAL_METER);

// A counter to indicate the access token cache miss
counter_metric!(ACCESS_TOKEN_CACHE_MISS, GLOBAL_METER);

// A counter to indicate the integrity check failures
counter_metric!(INTEGRITY_CHECK_FAILED, GLOBAL_METER);

// FRM (Fraud Risk Management) metrics
counter_metric!(FRM_FAILURE, GLOBAL_METER);

// Network Tokenization metrics
histogram_metric_f64!(GENERATE_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(FETCH_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(DELETE_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(CHECK_NETWORK_TOKEN_STATUS_TIME, GLOBAL_METER);
histogram_metric_f64!(FETCH_ALTID_TIME, GLOBAL_METER);

// A counter to indicate allowed payment method types mismatch
counter_metric!(PAYMENT_METHOD_TYPES_MISCONFIGURATION_METRIC, GLOBAL_METER);

// Merchant advice code config lookup metrics
counter_metric!(MERCHANT_ADVICE_CODE_CONFIG_MISS, GLOBAL_METER);

// Config Fetch Metrics
counter_metric!(CONFIG_DATABASE_FETCH, GLOBAL_METER); // When fetched from database
counter_metric!(CONFIG_DEFAULT_FALLBACK, GLOBAL_METER); // When defaulted to application default

// Payment Method (modular service) business-level metrics
counter_metric!(PAYMENT_METHOD_OPS_COUNT, GLOBAL_METER);
histogram_metric_f64!(PAYMENT_METHOD_OPERATION_DURATION, GLOBAL_METER);

counter_metric!(PAYMENT_METHOD_SESSION_OPS_COUNT, GLOBAL_METER);
histogram_metric_f64!(PAYMENT_METHOD_SESSION_OPERATION_DURATION, GLOBAL_METER);
counter_metric!(SUCCESSFUL_PAYMENT_METHOD_SESSION_CONFIRM, GLOBAL_METER);

// v2 vault call latency (payment_methods modular service) - distinct from the
// v1-only CARD_ADD_TIME/CARD_GET_TIME/CARD_DELETE_TIME, which instrument the
// legacy add_card_to_locker/delete_card_from_locker functions, not vault::call_to_vault.
histogram_metric_f64!(VAULT_ADD_TIME, GLOBAL_METER);
histogram_metric_f64!(VAULT_GET_TIME, GLOBAL_METER);
histogram_metric_f64!(VAULT_DELETE_TIME, GLOBAL_METER);
histogram_metric_f64!(VAULT_FINGERPRINT_TIME, GLOBAL_METER);
counter_metric!(VAULT_CALL_FAILURES, GLOBAL_METER);

// Encryption/keymanager latency for payment_methods operations
histogram_metric_f64!(PAYMENT_METHOD_CRYPTO_DURATION, GLOBAL_METER);
