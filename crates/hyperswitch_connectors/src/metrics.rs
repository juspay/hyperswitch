//! Metrics interface

use router_env::{counter_metric, global_meter, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(RESPONSE_DESERIALIZATION_FAILURE, GLOBAL_METER);
