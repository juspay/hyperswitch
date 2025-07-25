pub mod bg_metrics_collector;
pub mod request;

use router_env::{counter_metric, global_meter, histogram_metric_f64};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(HEALTH_METRIC, GLOBAL_METER); // No. of health API hits
counter_metric!(KV_MISS, GLOBAL_METER); // No. of KV misses

// API Level Metrics
counter_metric!(REQUESTS_RECEIVED, GLOBAL_METER);
histogram_metric_f64!(REQUEST_TIME, GLOBAL_METER);

// Operation Level Metrics
counter_metric!(PAYMENT_OPS_COUNT, GLOBAL_METER);

counter_metric!(PAYMENT_COUNT, GLOBAL_METER);
counter_metric!(SUCCESSFUL_PAYMENT, GLOBAL_METER);
//TODO: This can be removed, added for payment list debugging
histogram_metric_f64!(PAYMENT_LIST_LATENCY, GLOBAL_METER);

counter_metric!(REFUND_COUNT, GLOBAL_METER);
counter_metric!(SUCCESSFUL_REFUND, GLOBAL_METER);

counter_metric!(PAYMENT_CANCEL_COUNT, GLOBAL_METER);
counter_metric!(SUCCESSFUL_CANCEL, GLOBAL_METER);

counter_metric!(MANDATE_COUNT, GLOBAL_METER);
counter_metric!(SUBSEQUENT_MANDATE_PAYMENT, GLOBAL_METER);

// Manual retry metrics
counter_metric!(MANUAL_RETRY_REQUEST_COUNT, GLOBAL_METER);
counter_metric!(MANUAL_RETRY_COUNT, GLOBAL_METER);
counter_metric!(MANUAL_RETRY_VALIDATION_FAILED, GLOBAL_METER);

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

// Network Tokenization metrics
histogram_metric_f64!(GENERATE_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(FETCH_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(DELETE_NETWORK_TOKEN_TIME, GLOBAL_METER);
histogram_metric_f64!(CHECK_NETWORK_TOKEN_STATUS_TIME, GLOBAL_METER);

// A counter to indicate allowed payment method types mismatch
counter_metric!(PAYMENT_METHOD_TYPES_MISCONFIGURATION_METRIC, GLOBAL_METER);

// AI chat metric to track number of chat request
counter_metric!(CHAT_REQUEST_COUNT, GLOBAL_METER);
