use diesel_models::observability::{
    alerts_intermediate::AlertsIntermediate, alerts_main::AlertsMain, raw_json::RawJson,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Copy, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Channel {
    Slack,
    Xyne,
}

#[derive(Debug, Serialize)]
pub struct AlertStateEntry {
    pub id_intermediate: uuid::Uuid,
    pub announcement_id: Option<uuid::Uuid>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: Option<serde_json::Value>,
    pub ts_slack: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub max_duration: Option<i32>,
    pub other_metrics: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub rca_metadata: Option<serde_json::Value>,
    pub group_id: Option<String>,
    pub priority: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertStateWrite {
    #[serde(default)]
    pub id_intermediate: Option<uuid::Uuid>,
    #[serde(default)]
    pub announcement_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub dimensions: Option<serde_json::Value>,
    #[serde(default)]
    pub ts_slack: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    #[serde(default)]
    pub max_duration: Option<i32>,
    #[serde(default)]
    pub other_metrics: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata_alert_details: Option<serde_json::Value>,
    #[serde(default)]
    pub rca_metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct LifecycleStateResponse {
    pub count: usize,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub alerts: Vec<AlertStateEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleStateWriteRequest {
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expected_last_updated_at: Option<PrimitiveDateTime>,
    pub alerts: Vec<AlertStateWrite>,
}

#[derive(Debug, Serialize)]
pub struct LifecycleStateSaveResponse {
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub stored: usize,
    pub removed: usize,
    pub id_intermediates: Vec<uuid::Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncementRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub dimensions: Option<Box<RawValue>>,
    #[serde(default)]
    pub ts_slack: Option<String>,
    #[serde(default)]
    pub duration: Option<i32>,
    #[serde(default)]
    pub sent: Option<bool>,
    #[serde(default)]
    pub critical: Option<bool>,
    #[serde(default)]
    pub rca_metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Option<Box<RawValue>>,
}

#[derive(Debug, Serialize)]
pub struct AnnouncementEntry {
    pub id: uuid::Uuid,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: Option<Box<RawValue>>,
    pub ts_slack: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    pub duration: Option<i32>,
    pub sent: Option<bool>,
    pub critical: Option<bool>,
    pub rca_metadata: Option<serde_json::Value>,
    pub metadata: Option<Box<RawValue>>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncementListRequest {
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct AnnouncementListResponse {
    pub count: usize,
    pub announcements: Vec<AnnouncementEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncementUpdateRequest {
    pub metadata: Box<RawValue>,
}

impl From<AlertsIntermediate> for AlertStateEntry {
    fn from(row: AlertsIntermediate) -> Self {
        Self {
            id_intermediate: row.id_intermediate,
            announcement_id: row.id,
            name: row.name,
            product: row.product,
            dimensions: row.dimensions,
            ts_slack: row.ts_slack,
            ts_alert: row.ts_alert,
            latest_ts_alert: row.latest_ts_alert,
            max_duration: row.max_duration,
            other_metrics: row.other_metrics,
            metadata: row.metadata,
            metadata_alert_details: row.metadata_alert_details,
            rca_metadata: row.rca_metadata,
            group_id: row.group_id,
            priority: row.priority,
            last_updated_at: row.last_updated_at,
            recovered_ts: row.recovered_ts,
        }
    }
}

impl From<AlertsMain> for AnnouncementEntry {
    fn from(row: AlertsMain) -> Self {
        Self {
            id: row.id,
            name: row.name,
            product: row.product,
            dimensions: row.dimensions.map(RawJson::into_raw),
            ts_slack: row.ts_slack,
            ts_alert: row.ts_alert,
            duration: row.duration,
            sent: row.sent,
            critical: row.critical,
            rca_metadata: row.rca_metadata,
            metadata: row.metadata.map(RawJson::into_raw),
            last_updated_at: row.last_updated_at,
        }
    }
}
