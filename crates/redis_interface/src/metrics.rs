use router_env::{counter_metric, gauge_metric, global_meter, histogram_metric_f64};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(REDIS_OPERATION_TOTAL, GLOBAL_METER);
counter_metric!(REDIS_OPERATION_FAILURES, GLOBAL_METER);
histogram_metric_f64!(REDIS_OPERATION_LATENCY_SECONDS, GLOBAL_METER);

gauge_metric!(REDIS_OPERATION_AVG_LATENCY_MS, GLOBAL_METER);
gauge_metric!(REDIS_OPERATION_MIN_LATENCY_MS, GLOBAL_METER);
gauge_metric!(REDIS_OPERATION_MAX_LATENCY_MS, GLOBAL_METER);
