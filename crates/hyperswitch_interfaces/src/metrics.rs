//! Metrics interface

use router_env::{counter_metric, global_meter, metrics_context, opentelemetry};

metrics_context!(CONTEXT);
global_meter!(GLOBAL_METER, "ROUTER_API");

counter_metric!(UNIMPLEMENTED_FLOW, GLOBAL_METER);

/// fn add attributes
pub fn add_attributes<T: Into<opentelemetry::Value>>(
    key: &'static str,
    value: T,
) -> opentelemetry::KeyValue {
    opentelemetry::KeyValue::new(key, value)
}
