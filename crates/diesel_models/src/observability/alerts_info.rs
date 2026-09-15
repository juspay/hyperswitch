//! Alert definitions: one row per alert, and the whole of its configuration.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alerts_info;

/// A new alert definition.
///
/// `id` and `last_updated_at` are absent so the database assigns them. A `None` field is left out
/// of the insert entirely, so `is_enabled` and `author` fall back to their column defaults.
#[derive(Clone, Debug, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = alerts_info)]
pub struct AlertsInfoNew {
    pub name: String,
    pub product: String,
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    pub default_channel: Option<String>,
    pub default_critical: Option<bool>,
    pub blacklist: Option<serde_json::Value>,
    pub snooze: Option<serde_json::Value>,
    pub history_window: Option<i32>,
    pub thresholds: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_enabled: Option<bool>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: Option<String>,
    pub approver: Option<String>,
}

/// A stored alert definition.
#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = alerts_info, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertsInfo {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub dimensions: Option<String>,
    pub period: Option<i32>,
    pub default_channel: Option<String>,
    pub default_critical: Option<bool>,
    pub blacklist: Option<serde_json::Value>,
    pub snooze: Option<serde_json::Value>,
    pub history_window: Option<i32>,
    pub thresholds: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub is_enabled: Option<bool>,
    pub comments: Option<serde_json::Value>,
    pub call_period: Option<i32>,
    pub author: Option<String>,
    pub approver: Option<String>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}
