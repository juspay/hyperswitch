pub use router_env::opentelemetry::KeyValue;
use router_env::{counter_metric, global_meter, metrics_context};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "CONNECTORS");

// Flow Specific Metrics

counter_metric!(CONNECTOR_RESPONSE_DESERIALIZATION_FAILURE, GLOBAL_METER);
