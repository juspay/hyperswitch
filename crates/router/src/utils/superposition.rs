//!
//! Utilities for converting between `open-feature` and `serde_json` types.
//!
//! This module provides a standard `From` implementation to bridge the gap
//! between the generic `Value` type provided by the OpenFeature SDK and the
//! `serde_json::Value` type that is used throughout the application for
//! JSON manipulation.
//!
//! This allows for clean, idiomatic conversion using `.into()` wherever
//! a value from the feature flag provider needs to be deserialized into a
//! specific application struct.

pub fn openfeature_value_to_json(val: open_feature::Value) -> serde_json::Value {
    match val {
        open_feature::Value::Bool(b) => serde_json::Value::Bool(b),
        open_feature::Value::Int(i) => serde_json::Value::Number(i.into()),
        open_feature::Value::Float(f) => serde_json::Number::from_f64(f)
            .map_or(serde_json::Value::Null, serde_json::Value::Number),
        open_feature::Value::String(s) => serde_json::Value::String(s),
        open_feature::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(openfeature_value_to_json).collect())
        }
        open_feature::Value::Struct(s) => {
            let map = s
                .fields
                .into_iter()
                .map(|(k, v)| (k, openfeature_value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}
