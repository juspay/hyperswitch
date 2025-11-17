use router_env::{counter_metric, global_meter, histogram_metric_f64};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(DELETE_FROM_LOCKER, GLOBAL_METER);
counter_metric!(CARD_LOCKER_FAILURES, GLOBAL_METER);
histogram_metric_f64!(CARD_DELETE_TIME, GLOBAL_METER);
histogram_metric_f64!(DELETE_NETWORK_TOKEN_TIME, GLOBAL_METER);