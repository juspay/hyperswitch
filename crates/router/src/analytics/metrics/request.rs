pub fn add_attributes<T: Into<router_env::opentelemetry::Value>>(
    key: &'static str,
    value: T,
) -> router_env::opentelemetry::KeyValue {
    router_env::opentelemetry::KeyValue::new(key, value)
}

#[cfg(any(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
#[inline]
pub async fn record_operation_time<F, R>(
    future: F,
    metric: &once_cell::sync::Lazy<router_env::opentelemetry::metrics::Histogram<f64>>,
    metric_name: &api_models::analytics::payments::PaymentMetrics,
    source: &crate::analytics::AnalyticsProvider,
) -> R
where
    F: futures::Future<Output = R>,
{
    let (result, time) = time_future(future).await;
    let attributes = &[
        add_attributes("metric_name", metric_name.to_string()),
        add_attributes(
            "source",
            match source {
                #[cfg(feature = "clickhouse_analytics")]
                crate::analytics::AnalyticsProvider::Clickhouse(_) => "Clickhouse",
                #[cfg(feature = "sqlx_analytics")]
                crate::analytics::AnalyticsProvider::Sqlx(_) => "Sqlx",
                #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
                crate::analytics::AnalyticsProvider::CombinedCkh(_, _) => "CombinedCkh",
                #[cfg(all(feature = "clickhouse_analytics", feature = "sqlx_analytics"))]
                crate::analytics::AnalyticsProvider::CombinedSqlx(_, _) => "CombinedSqlx",
            },
        ),
    ];
    let value = time.as_secs_f64();
    metric.record(&super::CONTEXT, value, attributes);

    router_env::logger::debug!("Attributes: {:?}, Time: {}", attributes, value);
    result
}

use std::time;

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

#[macro_export]
macro_rules! histogram_metric {
    ($name:ident, $meter:ident) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_histogram(stringify!($name)).init());
    };
    ($name:ident, $meter:ident, $description:literal) => {
        pub(crate) static $name: once_cell::sync::Lazy<
            $crate::opentelemetry::metrics::Histogram<u64>,
        > = once_cell::sync::Lazy::new(|| $meter.u64_histogram($description).init());
    };
}
