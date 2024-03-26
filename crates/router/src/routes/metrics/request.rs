use router_env::opentelemetry;

use super::utils as metric_utils;
use crate::services::ApplicationResponse;

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

pub fn add_attributes<T: Into<router_env::opentelemetry::Value>>(
    key: &'static str,
    value: T,
) -> router_env::opentelemetry::KeyValue {
    router_env::opentelemetry::KeyValue::new(key, value)
}

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

pub fn track_response_status_code<Q>(response: &ApplicationResponse<Q>) -> i64 {
    match response {
        ApplicationResponse::Json(_)
        | ApplicationResponse::StatusOk
        | ApplicationResponse::TextPlain(_)
        | ApplicationResponse::Form(_)
        | ApplicationResponse::PaymentLinkForm(_)
        | ApplicationResponse::FileData(_)
        | ApplicationResponse::JsonWithHeaders(_) => 200,
        ApplicationResponse::JsonForRedirection(_) => 302,
    }
}
