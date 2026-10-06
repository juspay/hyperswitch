//! Metrics interface

use router_env::{counter_metric, global_meter};

global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(UNIMPLEMENTED_FLOW, GLOBAL_METER);

counter_metric!(CONNECTOR_CALL_COUNT, GLOBAL_METER); // Attributes needed

counter_metric!(RESPONSE_DESERIALIZATION_FAILURE, GLOBAL_METER);
counter_metric!(CONNECTOR_ERROR_RESPONSE_COUNT, GLOBAL_METER);
// Connector Level Metric
counter_metric!(REQUEST_BUILD_FAILURE, GLOBAL_METER);
