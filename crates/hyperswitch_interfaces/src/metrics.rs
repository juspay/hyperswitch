//! Metrics interface

use router_env::{counter_metric, global_meter, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(UNIMPLEMENTED_FLOW, GLOBAL_METER);
