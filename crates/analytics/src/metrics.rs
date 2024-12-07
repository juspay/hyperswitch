use router_env::{global_meter, histogram_metric_f64, histogram_metric_u64};

global_meter!(GLOBAL_METER, "ROUTER_API");

histogram_metric_f64!(METRIC_FETCH_TIME, GLOBAL_METER);
histogram_metric_u64!(BUCKETS_FETCHED, GLOBAL_METER);

pub mod request;
