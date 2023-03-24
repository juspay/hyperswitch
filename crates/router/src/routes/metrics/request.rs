use super::utils as metric_utils;

pub async fn record_request_time_metric<F, R>(
    future: F,
    flow: impl router_env::types::FlowMetric,
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
pub async fn record_card_operation_time<F, R>(
    future: F,
    metric: &once_cell::sync::Lazy<router_env::opentelemetry::metrics::Histogram<f64>>,
) -> R
where
    F: futures::Future<Output = R>,
{
    let (result, time) = metric_utils::time_future(future).await;
    metric.record(&super::CONTEXT, time.as_secs_f64(), &[]);
    result
}

pub fn add_attributes<T: Into<router_env::opentelemetry::Value>>(
    key: &'static str,
    value: T,
) -> router_env::opentelemetry::KeyValue {
    router_env::opentelemetry::KeyValue::new(key, value)
}
