use diesel_models::observability::{
    alerts_intermediate::AlertStateRow, alerts_main::AnnouncementRow, raw_json::RawJson,
};
use error_stack::report;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};
use crate::errors::{ObservabilityApiResult, ObservabilityError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Slack,
    Xyne,
}

impl Channel {
    pub fn from_path(segment: &str) -> ObservabilityApiResult<Self> {
        match segment {
            "slack" => Ok(Self::Slack),
            "xyne" => Ok(Self::Xyne),
            other => Err(report!(ObservabilityError::UnknownChannel {
                channel: other.to_owned(),
            }))?,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::Xyne => "xyne",
        }
    }

    pub(crate) fn lock_key(self) -> i32 {
        match self {
            Self::Slack => 1,
            Self::Xyne => 2,
        }
    }
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
    pub status: ReadStatus,
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
    pub status: WriteStatus,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub alerts: usize,
    pub removed: usize,
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

#[derive(Debug, Serialize)]
pub struct AnnouncementSaveResponse {
    pub status: WriteStatus,
    pub announcement: AnnouncementEntry,
}

impl From<AlertStateRow> for AlertStateEntry {
    fn from(row: AlertStateRow) -> Self {
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

impl From<AnnouncementRow> for AnnouncementEntry {
    fn from(row: AnnouncementRow) -> Self {
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
