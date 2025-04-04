//! metric utility functions

use std::time;

use router_env::opentelemetry;

/// Record the time taken by the future to execute
#[inline]
pub async fn time_future<F, R>(future: F) -> (R, time::Duration)
where
    F: futures::Future<Output = R>,
{
    let start = time::Instant::now();
    let result = future.await;
    let time_spent = start.elapsed();
    (result, time_spent)
}

/// Record the time taken (in seconds) by the operation for the given context
#[inline]
pub async fn record_operation_time<F, R>(
    future: F,
    metric: &opentelemetry::metrics::Histogram<f64>,
    key_value: &[opentelemetry::KeyValue],
) -> R
where
    F: futures::Future<Output = R>,
{
    let (result, time) = time_future(future).await;
    metric.record(time.as_secs_f64(), key_value);
    result
}
