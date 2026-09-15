//! Alert definitions, as the observability service's `/alerts/info` routes accept and return them.

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// The body of `POST /alerts/info`.
///
/// Only `name` and `product` are required. An omitted `is_enabled` or `author` takes the database
/// default rather than being stored as `null`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertsInfoCreateRequest {
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
#[derive(Clone, Debug, Serialize)]
pub struct AlertsInfoResponse {
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
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}
