use diesel_models::observability::{
    alerts_intermediate::{AlertsIntermediate, AlertsIntermediateNew},
    alerts_main::{AlertsMain, AlertsMainNew},
    raw_json::RawJson,
};
use error_stack::{report, ResultExt};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{not_blank, within_width};
use crate::errors::{ObservabilityApiResult, ObservabilityError};

const NAME_MAX_CHARS: usize = 64;

const TS_SLACK_MAX_CHARS: usize = 255;

const EMPTY_LIST: &str = "[]";

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
    pub name: String,
    pub product: String,
    pub dimensions: serde_json::Value,
    pub ts_slack: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub latest_ts_alert: PrimitiveDateTime,
    pub max_duration: i32,
    pub other_metrics: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub rca_metadata: serde_json::Value,
    pub group_id: String,
    pub priority: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertStateWrite {
    pub id_intermediate: Option<uuid::Uuid>,
    pub announcement_id: Option<uuid::Uuid>,
    pub name: String,
    pub product: String,
    pub dimensions: Option<serde_json::Value>,
    pub ts_slack: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub max_duration: Option<i32>,
    pub other_metrics: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub rca_metadata: Option<serde_json::Value>,
    pub group_id: Option<String>,
    pub priority: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
}

impl AlertStateWrite {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        not_blank("name", &self.name)?;
        not_blank("product", &self.product)?;
        within_width("name", Some(&self.name), NAME_MAX_CHARS)?;
        within_width("product", Some(&self.product), NAME_MAX_CHARS)?;
        within_width("group_id", self.group_id.as_deref(), NAME_MAX_CHARS)?;
        within_width("priority", self.priority.as_deref(), NAME_MAX_CHARS)?;
        within_width("ts_slack", self.ts_slack.as_deref(), TS_SLACK_MAX_CHARS)?;
        not_before_unix_epoch("ts_alert", self.ts_alert)?;
        not_before_unix_epoch("latest_ts_alert", self.latest_ts_alert)?;
        not_before_unix_epoch("recovered_ts", self.recovered_ts)
    }

    pub fn into_insertable(
        self,
        id_intermediate: uuid::Uuid,
        channel: &str,
        now: PrimitiveDateTime,
    ) -> AlertsIntermediateNew {
        AlertsIntermediateNew {
            id_intermediate,
            channel: channel.to_owned(),
            id: self.announcement_id,
            name: self.name,
            product: self.product,
            dimensions: self
                .dimensions
                .unwrap_or_else(|| serde_json::Value::Array(Vec::new())),
            ts_slack: self.ts_slack,
            ts_alert: self.ts_alert.unwrap_or(now),
            latest_ts_alert: self.latest_ts_alert.unwrap_or(now),
            max_duration: self.max_duration.unwrap_or_default(),
            other_metrics: self.other_metrics,
            metadata: self.metadata,
            metadata_alert_details: self.metadata_alert_details,
            rca_metadata: self
                .rca_metadata
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
            group_id: self.group_id.unwrap_or_default(),
            priority: self.priority.unwrap_or_default(),
            last_updated_at: now,
            recovered_ts: self.recovered_ts,
        }
    }
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
    pub name: String,
    pub product: String,
    pub dimensions: Option<RawJson>,
    pub ts_slack: Option<String>,
    pub duration: Option<i32>,
    pub sent: Option<bool>,
    pub critical: Option<bool>,
    pub rca_metadata: Option<serde_json::Value>,
    pub metadata: Option<RawJson>,
}

impl AnnouncementRequest {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        not_blank("name", &self.name)?;
        not_blank("product", &self.product)?;
        within_width("name", Some(&self.name), NAME_MAX_CHARS)?;
        within_width("product", Some(&self.product), NAME_MAX_CHARS)?;
        within_width("ts_slack", self.ts_slack.as_deref(), TS_SLACK_MAX_CHARS)
    }

    pub fn into_insertable(
        self,
        id: uuid::Uuid,
        channel: &str,
        now: PrimitiveDateTime,
    ) -> ObservabilityApiResult<AlertsMainNew> {
        Ok(AlertsMainNew {
            id,
            channel: channel.to_owned(),
            name: self.name,
            product: self.product,
            dimensions: or_empty_list(self.dimensions)?,
            ts_slack: self.ts_slack,
            ts_alert: now,
            duration: self.duration.unwrap_or_default(),
            sent: self.sent.unwrap_or_default(),
            critical: self.critical.unwrap_or_default(),
            rca_metadata: self
                .rca_metadata
                .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new())),
            metadata: self.metadata,
            last_updated_at: now,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AnnouncementEntry {
    pub id: uuid::Uuid,
    pub name: String,
    pub product: String,
    pub dimensions: RawJson,
    pub ts_slack: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    pub duration: i32,
    pub sent: bool,
    pub critical: bool,
    pub rca_metadata: serde_json::Value,
    pub metadata: Option<RawJson>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncementListRequest {
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end: Option<PrimitiveDateTime>,
}

impl AnnouncementListRequest {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        not_before_unix_epoch("start", self.start)?;
        not_before_unix_epoch("end", self.end)
    }
}

#[derive(Debug, Serialize)]
pub struct AnnouncementListResponse {
    pub count: usize,
    pub announcements: Vec<AnnouncementEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncementUpdateRequest {
    pub metadata: RawJson,
}

impl AnnouncementUpdateRequest {
    pub fn metadata_patch(
        &self,
    ) -> ObservabilityApiResult<serde_json::Map<String, serde_json::Value>> {
        serde_json::from_str(self.metadata.get()).change_context(
            ObservabilityError::InvalidRequestData {
                message: "metadata must be an object".to_owned(),
            },
        )
    }
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
            dimensions: row.dimensions,
            ts_slack: row.ts_slack,
            ts_alert: row.ts_alert,
            duration: row.duration,
            sent: row.sent,
            critical: row.critical,
            rca_metadata: row.rca_metadata,
            metadata: row.metadata,
            last_updated_at: row.last_updated_at,
        }
    }
}

fn or_empty_list(column: Option<RawJson>) -> ObservabilityApiResult<RawJson> {
    column.map_or_else(
        || {
            serde_json::from_str(EMPTY_LIST)
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable("Failed to build an empty JSON list")
        },
        Ok,
    )
}

fn not_before_unix_epoch(
    field_name: &'static str,
    value: Option<PrimitiveDateTime>,
) -> ObservabilityApiResult<()> {
    common_utils::fp_utils::when(
        value.is_some_and(|value| value.assume_utc() < time::OffsetDateTime::UNIX_EPOCH),
        || Err(report!(ObservabilityError::InvalidDataValue { field_name })),
    )
}
