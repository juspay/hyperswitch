use router_env::{counter_metric, global_meter, histogram_metric_f64};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(REQUEST_BUILD_FAILURE, GLOBAL_METER);

histogram_metric_f64!(EXTERNAL_REQUEST_TIME, GLOBAL_METER);

counter_metric!(AUTO_RETRY_CONNECTION_CLOSED, GLOBAL_METER);
