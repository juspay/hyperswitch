//! Metrics interface

use router_env::{counter_metric, global_meter};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(CONNECTOR_RESPONSE_DESERIALIZATION_FAILURE, GLOBAL_METER);