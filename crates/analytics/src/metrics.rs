use router_env::{global_meter, histogram_metric, histogram_metric_u64, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

histogram_metric!(METRIC_FETCH_TIME, GLOBAL_METER);
histogram_metric_u64!(BUCKETS_FETCHED, GLOBAL_METER);

pub mod request;
