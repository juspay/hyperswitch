//! Alert definitions: what an alert is configured to watch, and how.
//!
//! These sit between the API types in `api_models` and the rows in `diesel_models`, so neither
//! [`crate::core`] nor [`crate::db`] callers have to know the other's shape. The conversions to and
//! from both live here.

use api_models::observability::alerts_info as api;
use diesel_models::observability::alerts_info as storage;
use time::PrimitiveDateTime;

/// An alert definition that has not been stored yet.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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

impl From<api::AlertsInfoCreateRequest> for AlertsInfoNew {
    fn from(request: api::AlertsInfoCreateRequest) -> Self {
        Self {
            name: request.name,
            product: request.product,
            dimensions: request.dimensions,
            period: request.period,
            default_channel: request.default_channel,
            default_critical: request.default_critical,
            blacklist: request.blacklist,
            snooze: request.snooze,
            history_window: request.history_window,
            thresholds: request.thresholds,
            metadata: request.metadata,
            is_enabled: request.is_enabled,
            comments: request.comments,
            call_period: request.call_period,
            author: request.author,
            approver: request.approver,
        }
    }
}

impl From<AlertsInfoNew> for storage::AlertsInfoNew {
    fn from(new: AlertsInfoNew) -> Self {
        Self {
            name: new.name,
            product: new.product,
            dimensions: new.dimensions,
            period: new.period,
            default_channel: new.default_channel,
            default_critical: new.default_critical,
            blacklist: new.blacklist,
            snooze: new.snooze,
            history_window: new.history_window,
            thresholds: new.thresholds,
            metadata: new.metadata,
            is_enabled: new.is_enabled,
            comments: new.comments,
            call_period: new.call_period,
            author: new.author,
            approver: new.approver,
        }
    }
}

impl From<storage::AlertsInfo> for AlertsInfo {
    fn from(row: storage::AlertsInfo) -> Self {
        Self {
            id: row.id,
            name: row.name,
            product: row.product,
            dimensions: row.dimensions,
            period: row.period,
            default_channel: row.default_channel,
            default_critical: row.default_critical,
            blacklist: row.blacklist,
            snooze: row.snooze,
            history_window: row.history_window,
            thresholds: row.thresholds,
            metadata: row.metadata,
            is_enabled: row.is_enabled,
            comments: row.comments,
            call_period: row.call_period,
            author: row.author,
            approver: row.approver,
            last_updated_at: row.last_updated_at,
        }
    }
}

impl From<AlertsInfo> for api::AlertsInfoResponse {
    fn from(alert: AlertsInfo) -> Self {
        Self {
            id: alert.id,
            name: alert.name,
            product: alert.product,
            dimensions: alert.dimensions,
            period: alert.period,
            default_channel: alert.default_channel,
            default_critical: alert.default_critical,
            blacklist: alert.blacklist,
            snooze: alert.snooze,
            history_window: alert.history_window,
            thresholds: alert.thresholds,
            metadata: alert.metadata,
            is_enabled: alert.is_enabled,
            comments: alert.comments,
            call_period: alert.call_period,
            author: alert.author,
            approver: alert.approver,
            last_updated_at: alert.last_updated_at,
        }
    }
}
