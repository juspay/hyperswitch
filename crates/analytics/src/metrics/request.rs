use std::time;

#[inline]
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
    let attributes = router_env::metric_attributes!(
        ("metric_name", metric_name.to_string()),
        ("source", source.to_string()),
    );
    let value = time.as_secs_f64();
    metric.record(value, attributes);

    router_env::logger::debug!("Attributes: {:?}, Time: {}", attributes, value);
    result
}

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
