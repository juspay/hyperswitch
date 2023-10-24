#[cfg(any(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
use router_env_oss::histogram_metric;
use router_env_oss::{global_meter, metrics_context};

#[cfg(any(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
use crate::histogram_metric as histogram_metric_u64;

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

#[cfg(any(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
histogram_metric!(METRIC_FETCH_TIME, GLOBAL_METER);
#[cfg(any(feature = "sqlx_analytics", feature = "clickhouse_analytics"))]
histogram_metric_u64!(BUCKETS_FETCHED, GLOBAL_METER);

pub mod request;
