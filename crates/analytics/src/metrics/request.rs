/// Adds attributes to a key-value store for OpenTelemetry tracing.
/// 
/// # Arguments
/// 
/// * `key` - The key to store the attribute under.
/// * `value` - The value to be stored for the provided key.
/// 
/// # Returns
/// 
/// A `router_env::opentelemetry::KeyValue` object containing the provided key and value.
pub fn add_attributes<T: Into<router_env::opentelemetry::Value>>(
    key: &'static str,
    value: T,
) -> router_env::opentelemetry::KeyValue {
    router_env::opentelemetry::KeyValue::new(key, value)
}

#[inline]
/// Asynchronously records the time taken for a given operation using OpenTelemetry metrics.
/// 
/// # Arguments
/// 
/// * `future` - A future representing the operation for which the time is being recorded.
/// * `metric` - A reference to an OpenTelemetry Histogram metric.
/// * `metric_name` - A reference to the name of the metric.
/// * `source` - A reference to the AnalyticsProvider from which the operation originates.
/// 
/// # Generic Parameters
/// 
/// * `F` - The type of the future representing the operation.
/// * `R` - The return type of the future.
/// * `T` - The type that can be converted to a string, representing the metric name.
/// 
/// # Returns
/// 
/// Returns the result of the future.
/// 
/// # Panics
/// 
/// This method will panic if the future panics during execution.
/// 
pub async fn record_operation_time<F, R, T>(
    future: F,
    metric: &once_cell::sync::Lazy<router_env::opentelemetry::metrics::Histogram<f64>>,
    metric_name: &T,
    source: &crate::AnalyticsProvider,
) -> R
where
    F: futures::Future<Output = R>,
    T: ToString,
{
    let (result, time) = time_future(future).await;
    let attributes = &[
        add_attributes("metric_name", metric_name.to_string()),
        add_attributes("source", source.to_string()),
    ];
    let value = time.as_secs_f64();
    metric.record(&super::CONTEXT, value, attributes);

    router_env::logger::debug!("Attributes: {:?}, Time: {}", attributes, value);
    result
}

use std::time;

#[inline]
/// Asynchronously measures the time it takes for the given future to complete and returns a tuple containing the result of the future and the duration of time it took to complete.
///
/// # Arguments
///
/// * `future` - A future that will be executed asynchronously.
///
/// # Returns
///
/// A tuple containing the result of the future and the duration of time it took to complete.
///
pub async fn time_future<F, R>(future: F) -> (R, time::Duration)
where
    F: futures::Future<Output = R>,
{
    let start = time::Instant::now();
    let result = future.await;
    let time_spent = start.elapsed();
    (result, time_spent)
}
