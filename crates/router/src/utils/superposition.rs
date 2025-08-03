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

use open_feature::Value;
use serde_json;

/// Implements the standard `From` trait to convert an `open_feature::Value`
/// into a `serde_json::Value`.
///
/// This conversion is recursive, handling nested objects and arrays, and it
/// allows the rest of the application to use the powerful `serde_json::from_value`
/// function to deserialize feature flag results into strongly-typed structs.

use serde_json;
#[derive(Debug)]
pub struct SerdeValue(pub serde_json::Value);

impl From<open_feature::Value> for serde_json::Value {
    fn from(val: open_feature::Value) -> Self {
        match val {
            Value::Null => serde_json::Value::Null,
            Value::Boolean(b) => serde_json::Value::Bool(b),
            Value::String(s) => serde_json::Value::String(s),
            Value::Integer(i) => serde_json::Value::Number(i.into()),
            Value::Double(f) => serde_json::Number::from_f64(f)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Into::into).collect())
            }
            Value::Object(obj) => {
                serde_json::Value::Object(
                    obj.into_iter().map(|(k, v)| (k, v.into())).collect()
                )
            }
        }
    }
}
