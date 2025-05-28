#[inline]
pub async fn record_operation_time<F, R, T>(
    future: F,
    metric: &router_env::opentelemetry::metrics::Histogram<f64>,
    metric_name: &T,
    source: &crate::AnalyticsProvider,
) -> R
where
    F: futures::Future<Output = R>,
    T: ToString,
{
    let (result, time) = common_utils::metrics::utils::time_future(future).await;
    let attributes = router_env::metric_attributes!(
        ("metric_name", metric_name.to_string()),
        ("source", source.to_string()),
    );
    let value = time.as_secs_f64();
    metric.record(value, attributes);

    router_env::logger::debug!("Attributes: {:?}, Time: {}", attributes, value);
    result
}
