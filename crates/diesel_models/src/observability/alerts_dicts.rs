//! The mappers dictionary: small named lists that alert jobs and the dashboard look up by
//! `(name, key_)`. Every save is a new version.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alerts_dicts;

/// A new version of a dictionary entry.
///
/// `id` is absent so the column default (`gen_random_uuid()`) assigns it. A `None` `username`
/// falls back to its column default rather than being stored as `null`.
#[derive(Clone, Debug, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = alerts_dicts)]
pub struct AlertsDictsNew {
    pub name: String,
    pub key_: String,
    pub product: serde_json::Value,
    pub values_: serde_json::Value,
    pub ts_created: PrimitiveDateTime,
    pub is_enabled: bool,
    pub username: Option<String>,
    pub metadata: serde_json::Value,
}

/// A stored version of a dictionary entry.
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = alerts_dicts, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertsDicts {
    pub id: uuid::Uuid,
    pub name: String,
    pub key_: String,
    pub product: Option<serde_json::Value>,
    pub values_: Option<serde_json::Value>,
    pub ts_created: Option<PrimitiveDateTime>,
    pub is_enabled: Option<bool>,
    pub username: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
