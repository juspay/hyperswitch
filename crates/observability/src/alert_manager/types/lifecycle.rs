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
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn body_of<T: Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn a_channel_is_the_path_segment_it_is_spelled_as() {
        assert_eq!(Channel::from_path("slack").unwrap(), Channel::Slack);
        assert_eq!(Channel::from_path("xyne").unwrap(), Channel::Xyne);
        assert_eq!(Channel::Xyne.as_str(), "xyne");
    }

    #[test]
    fn an_unknown_channel_is_rejected_rather_than_defaulted() {
        assert!(Channel::from_path("Slack").is_err());
        assert!(Channel::from_path("").is_err());
    }

    #[test]
    fn the_two_channels_lock_separately() {
        assert_ne!(Channel::Slack.lock_key(), Channel::Xyne.lock_key());
    }

    #[test]
    fn empty_state_says_so_rather_than_returning_an_empty_body() {
        let body = body_of(&LifecycleStateResponse {
            status: ReadStatus::Absent,
            last_updated_at: None,
            alerts: Vec::new(),
        });

        assert_eq!(body["status"], "absent");
        assert_eq!(body["alerts"], serde_json::json!([]));
        assert!(body["last_updated_at"].is_null());
    }

    #[test]
    fn a_state_row_carries_every_field_even_when_the_columns_are_null() {
        let body = body_of(&AlertStateEntry::from(AlertStateRow {
            id_intermediate: uuid::Uuid::nil(),
            id: None,
            name: None,
            product: None,
            dimensions: None,
            ts_slack: None,
            ts_alert: None,
            latest_ts_alert: None,
            max_duration: None,
            other_metrics: None,
            metadata: None,
            metadata_alert_details: None,
            rca_metadata: None,
            group_id: None,
            priority: None,
            last_updated_at: None,
            recovered_ts: None,
        }));

        for field in [
            "announcement_id",
            "name",
            "product",
            "dimensions",
            "ts_slack",
            "ts_alert",
            "latest_ts_alert",
            "max_duration",
            "other_metrics",
            "metadata",
            "metadata_alert_details",
            "rca_metadata",
            "group_id",
            "priority",
            "last_updated_at",
            "recovered_ts",
        ] {
            assert!(
                body.get(field).is_some_and(serde_json::Value::is_null),
                "{field} was omitted"
            );
        }
    }

    #[test]
    fn the_announcement_reference_is_named_for_what_it_points_at() {
        let announcement = uuid::Uuid::now_v7();
        let body = body_of(&AlertStateEntry::from(AlertStateRow {
            id_intermediate: uuid::Uuid::nil(),
            id: Some(announcement),
            name: None,
            product: None,
            dimensions: None,
            ts_slack: None,
            ts_alert: None,
            latest_ts_alert: None,
            max_duration: None,
            other_metrics: None,
            metadata: None,
            metadata_alert_details: None,
            rca_metadata: None,
            group_id: None,
            priority: None,
            last_updated_at: None,
            recovered_ts: None,
        }));

        assert_eq!(body["announcement_id"], announcement.to_string());
        assert!(body.get("id").is_none());
    }

    #[test]
    fn a_write_without_a_precondition_asserts_the_state_was_empty() {
        let request: LifecycleStateWriteRequest =
            serde_json::from_str(r#"{"alerts": []}"#).unwrap();

        assert!(request.expected_last_updated_at.is_none());
        assert!(request.alerts.is_empty());
    }

    #[test]
    fn an_unknown_field_is_rejected_rather_than_dropped() {
        let error =
            serde_json::from_str::<LifecycleStateWriteRequest>(r#"{"alerts": [], "runs": 4}"#)
                .unwrap_err();

        assert!(error.to_string().contains("runs"));
    }

    #[test]
    fn an_announcements_stored_json_keeps_its_key_order() {
        let request: AnnouncementRequest =
            serde_json::from_str(r#"{"dimensions": {"b": 1, "a": [2, 3]}}"#).unwrap();

        let stored = RawJson::from(request.dimensions.unwrap());
        assert_eq!(stored.get(), r#"{"b": 1, "a": [2, 3]}"#);

        let entry = AnnouncementEntry::from(AnnouncementRow {
            id: uuid::Uuid::nil(),
            name: None,
            product: None,
            dimensions: Some(stored),
            ts_slack: None,
            ts_alert: None,
            duration: None,
            sent: None,
            critical: None,
            rca_metadata: None,
            metadata: None,
            last_updated_at: None,
        });

        assert_eq!(
            body_of(&entry)["dimensions"].to_string(),
            r#"{"b":1,"a":[2,3]}"#
        );
    }

    #[test]
    fn an_announcement_can_be_delivered_without_a_thread() {
        let request: AnnouncementRequest =
            serde_json::from_str(r#"{"sent": true, "ts_slack": null}"#).unwrap();

        assert_eq!(request.sent, Some(true));
        assert!(request.ts_slack.is_none());
    }
}
