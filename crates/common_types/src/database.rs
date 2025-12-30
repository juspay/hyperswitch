//! Types that are used for database interactions.

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Json,
};
use masking::{ExposeInterface, Secret};
use serde_json::Value;

/// A newtype wrapper for `Secret<serde_json::Value>` that merges JSON values instead of overwriting
/// them.
#[derive(
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
    diesel::AsExpression,
    diesel::FromSqlRow,
)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct JsonbMerge(Secret<Value>);

impl JsonbMerge {
    /// Create a new `JsonbMerge` from a `serde_json::Value`.
    pub fn new(value: Value) -> Self {
        Self(Secret::new(value))
    }

    /// Create a new `JsonbMerge` from an optional `serde_json::Value`.
    pub fn from_optional(value: Option<Value>) -> Option<Self> {
        value.map(Self::new)
    }

    /// Merge the given `serde_json::Value` with the existing value.
    pub fn merge(&mut self, value: Value) {
        // The `json-patch` crate is not used here because it does not handle merging of nested
        // objects correctly.
        // For example, if the existing value is `{"a": {"b": 1}}` and the new value is
        // `{"a": {"c": 2}}`, the `json-patch` crate will replace the value of `a` with `{"c": 2}`
        // instead of merging the two values to produce `{"a": {"b": 1, "c": 2}}`.
        fn merge_values(existing: &mut Value, new: Value) {
            match (existing, new) {
                (Value::Object(ref mut existing), Value::Object(new)) => {
                    for (key, value) in new {
                        merge_values(existing.entry(key).or_insert(Value::Null), value);
                    }
                }
                (existing, new) => *existing = new,
            }
        }

        let mut existing_value = self.0.clone().expose();
        merge_values(&mut existing_value, value);
        self.0 = Secret::new(existing_value);
    }

    /// Expose the inner `Secret<serde_json::Value>`.
    pub fn into_inner(self) -> Secret<Value> {
        self.0
    }
}

impl<DB> ToSql<Json, DB> for JsonbMerge
where
    DB: Backend,
    Secret<Value>: ToSql<Json, DB>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> FromSql<Json, DB> for JsonbMerge
where
    DB: Backend,
    Secret<Value>: FromSql<Json, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let secret_value = Secret::<Value>::from_sql(bytes)?;
        Ok(Self(secret_value))
    }
}

impl From<JsonbMerge> for serde_json::Value {
    fn from(value: JsonbMerge) -> Self {
        value.0.expose()
    }
}
