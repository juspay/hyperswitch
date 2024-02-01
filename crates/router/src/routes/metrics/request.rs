use router_env::opentelemetry;

use super::utils as metric_utils;
use crate::services::ApplicationResponse;

/// Asynchronously records the request time metric for the given future and flow. It adds the request type to the flow metric, records the time taken by the future to complete, and adds the request type to the recorded request time metric. Finally, it returns the result of the future.
pub async fn record_request_time_metric<F, R>(
    future: F,
    flow: &impl router_env::types::FlowMetric,
) -> R
where
    F: futures::Future<Output = R>,
{
    let key = "request_type";
    super::REQUESTS_RECEIVED.add(&super::CONTEXT, 1, &[add_attributes(key, flow.to_string())]);
    let (result, time) = metric_utils::time_future(future).await;
    super::REQUEST_TIME.record(
        &super::CONTEXT,
        time.as_secs_f64(),
        &[add_attributes(key, flow.to_string())],
    );
    result
}

#[inline]
/// Asynchronously records the operation time of a given future, updates the specified metric with the recorded time, and returns the result of the future.
///
/// # Arguments
///
/// * `future` - The future whose operation time will be recorded
/// * `metric` - The metric to update with the recorded time
/// * `key_value` - Additional key-value pairs to associate with the recorded time
///
/// # Generic
///
/// * `F` - The type of the future
/// * `R` - The type of the result returned by the future
///
/// # Returns
///
/// The result of the future
pub async fn record_operation_time<F, R>(
    future: F,
    metric: &once_cell::sync::Lazy<router_env::opentelemetry::metrics::Histogram<f64>>,
    key_value: &[opentelemetry::KeyValue],
) -> R
where
    F: futures::Future<Output = R>,
{
    let (result, time) = metric_utils::time_future(future).await;
    metric.record(&super::CONTEXT, time.as_secs_f64(), key_value);
    result
}

/// Adds attributes to the OpenTelemetry KeyValue object with the specified key and value.
pub fn add_attributes<T: Into<router_env::opentelemetry::Value>>(
    key: &'static str,
    value: T,
) -> router_env::opentelemetry::KeyValue {
    router_env::opentelemetry::KeyValue::new(key, value)
}

/// Adds status code metrics to the request status.
///
/// # Arguments
///
/// * `status_code` - The status code to be added to the metrics.
/// * `flow` - The flow of the request.
/// * `merchant_id` - The ID of the merchant associated with the request.
///
pub fn status_code_metrics(status_code: i64, flow: String, merchant_id: String) {
    super::REQUEST_STATUS.add(
        &super::CONTEXT,
        1,
        &[
            add_attributes("status_code", status_code),
            add_attributes("flow", flow),
            add_attributes("merchant_id", merchant_id),
        ],
    )
}
/// Returns the HTTP status code of the given ApplicationResponse.
pub fn track_response_status_code<Q>(response: &ApplicationResponse<Q>) -> i64 {
    match response {
        ApplicationResponse::Json(_)
        | ApplicationResponse::StatusOk
        | ApplicationResponse::TextPlain(_)
        | ApplicationResponse::Form(_)
        | ApplicationResponse::PaymenkLinkForm(_)
        | ApplicationResponse::FileData(_)
        | ApplicationResponse::JsonWithHeaders(_) => 200,
        ApplicationResponse::JsonForRedirection(_) => 302,
    }
}
