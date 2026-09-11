//! The wire contract for alert lifecycle state and the announcements made about it.

use diesel_models::observability::{
    alerts_intermediate::AlertStateRow, alerts_main::AnnouncementRow, raw_json::RawJson,
};
use error_stack::report;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};
use crate::errors::{ObservabilityApiResult, ObservabilityError};

/// Which delivery channel's lifecycle a request is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// `alerts_main` and `alerts_intermediate`.
    Slack,
    /// `alerts_main_xyne` and `alerts_intermediate_xyne`.
    Xyne,
}

impl Channel {
    /// Resolve the channel a path names.
    pub fn from_path(segment: &str) -> ObservabilityApiResult<Self> {
        match segment {
            "slack" => Ok(Self::Slack),
            "xyne" => Ok(Self::Xyne),
            other => Err(report!(ObservabilityError::UnknownChannel {
                channel: other.to_owned(),
            }))?,
        }
    }

    /// The segment this channel is spelled as.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Slack => "slack",
            Self::Xyne => "xyne",
        }
    }

    /// The channel's half of the advisory lock key that serialises whole-state writes.
    pub(crate) fn lock_key(self) -> i32 {
        match self {
            Self::Slack => 1,
            Self::Xyne => 2,
        }
    }
}

/// One lifecycle row, as the alert manager reads it back.
#[derive(Debug, Serialize)]
pub struct AlertStateEntry {
    /// The row's identity.
    pub id_intermediate: uuid::Uuid,
    /// The announcement this episode was announced under, or `null` if nothing has been said yet.
    pub announcement_id: Option<uuid::Uuid>,
    /// The detector that raised it.
    pub name: Option<String>,
    /// The product it was raised for.
    pub product: Option<String>,
    /// What the alert is about — merchant, profile, connector.
    pub dimensions: Option<serde_json::Value>,
    /// The provider's thread id, carried for the life of the episode so reminders and the eventual recovery land under the first announcement rather than as new top-level messages.
    pub ts_slack: Option<String>,
    /// When this episode started.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    /// When it was last seen firing.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    /// How long the episode has run, as the caller last computed it.
    pub max_duration: Option<i32>,
    /// Whatever else the detector reported.
    pub other_metrics: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    pub metadata: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    pub metadata_alert_details: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    pub rca_metadata: Option<serde_json::Value>,
    /// The alert key:
    pub group_id: Option<String>,
    /// The severity the detector assigned.
    pub priority: Option<String>,
    /// When this service last wrote the row.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    /// When the episode recovered.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
}

/// One lifecycle row, as a whole-state write sends it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertStateWrite {
    /// The row to update, or absent to create one.
    #[serde(default)]
    pub id_intermediate: Option<uuid::Uuid>,
    /// The announcement this episode was announced under.
    #[serde(default)]
    pub announcement_id: Option<uuid::Uuid>,
    /// The detector that raised it.
    #[serde(default)]
    pub name: Option<String>,
    /// The product it was raised for.
    #[serde(default)]
    pub product: Option<String>,
    /// What the alert is about — merchant, profile, connector.
    #[serde(default)]
    pub dimensions: Option<serde_json::Value>,
    /// The provider's thread id.
    #[serde(default)]
    pub ts_slack: Option<String>,
    /// When this episode started.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    /// When it was last seen firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    /// How long the episode has run.
    #[serde(default)]
    pub max_duration: Option<i32>,
    /// Whatever else the detector reported.
    #[serde(default)]
    pub other_metrics: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub metadata_alert_details: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub rca_metadata: Option<serde_json::Value>,
    /// The alert key.
    #[serde(default)]
    pub group_id: Option<String>,
    /// The severity the detector assigned.
    #[serde(default)]
    pub priority: Option<String>,
    /// When the episode recovered, or absent while it is still firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
}

/// What `GET /alerts/lifecycle/{channel}/state` returns.
#[derive(Debug, Serialize)]
pub struct LifecycleStateResponse {
    /// Whether any state is held.
    pub status: ReadStatus,
    /// The newest `last_updated_at` across the rows returned, and the value to send back as [`LifecycleStateWriteRequest::expected_last_updated_at`].
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    /// Every state row, oldest episode first.
    pub alerts: Vec<AlertStateEntry>,
}

/// The body of `POST /alerts/lifecycle/{channel}/state`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleStateWriteRequest {
    /// The `last_updated_at` this service returned when the caller read the state it is now replacing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expected_last_updated_at: Option<PrimitiveDateTime>,
    /// The whole of the state after this write.
    pub alerts: Vec<AlertStateWrite>,
}

/// What `POST /alerts/lifecycle/{channel}/state` returns.
#[derive(Debug, Serialize)]
pub struct LifecycleStateSaveResponse {
    /// Always [`WriteStatus::Saved`]; a write that did not apply is an error status, not this one.
    pub status: WriteStatus,
    /// The precondition for the next write.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
    /// How many rows the state now holds.
    pub alerts: usize,
    /// How many rows this write removed.
    pub removed: usize,
}

/// The body of `POST /alerts/lifecycle/{channel}/announcements`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnouncementRequest {
    /// The detector that raised it.
    #[serde(default)]
    pub name: Option<String>,
    /// The product it was raised for.
    #[serde(default)]
    pub product: Option<String>,
    /// What the alert was about.
    #[serde(default)]
    pub dimensions: Option<Box<RawValue>>,
    /// The provider's thread id, as the provider returned it.
    #[serde(default)]
    pub ts_slack: Option<String>,
    /// How long the episode had run when this went out.
    #[serde(default)]
    pub duration: Option<i32>,
    /// Whether the message was delivered.
    #[serde(default)]
    pub sent: Option<bool>,
    /// Whether it was announced as critical.
    #[serde(default)]
    pub critical: Option<bool>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub rca_metadata: Option<serde_json::Value>,
    /// Free-form.
    #[serde(default)]
    pub metadata: Option<Box<RawValue>>,
}

/// One announcement, as it is stored.
#[derive(Debug, Serialize)]
pub struct AnnouncementEntry {
    /// The announcement's id, minted here.
    pub id: uuid::Uuid,
    /// The detector that raised it.
    pub name: Option<String>,
    /// The product it was raised for.
    pub product: Option<String>,
    /// Returned as the bytes that were saved.
    pub dimensions: Option<Box<RawValue>>,
    /// The provider's thread id.
    pub ts_slack: Option<String>,
    /// When the announcement was recorded, by this service's clock.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    /// How long the episode had run when this went out.
    pub duration: Option<i32>,
    /// Whether the message was delivered.
    pub sent: Option<bool>,
    /// Whether it was announced as critical.
    pub critical: Option<bool>,
    /// Free-form, uninterpreted here.
    pub rca_metadata: Option<serde_json::Value>,
    /// Returned as the bytes that were saved.
    pub metadata: Option<Box<RawValue>>,
    /// When this service last wrote the row.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
}

/// What `POST /alerts/lifecycle/{channel}/announcements` returns.
#[derive(Debug, Serialize)]
pub struct AnnouncementSaveResponse {
    /// Always [`WriteStatus::Saved`].
    pub status: WriteStatus,
    /// The stored announcement, read back from the row that was written — so the caller has the id to reference without guessing it.
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
            // `into_raw` moves the stored bytes onto the wire, without parsing and re-encoding them on the way.
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

    /// A `404` rather than a fallback to one of the two:
    #[test]
    fn an_unknown_channel_is_rejected_rather_than_defaulted() {
        assert!(Channel::from_path("Slack").is_err());
        assert!(Channel::from_path("").is_err());
    }

    /// The two locks must never collide, or one channel's write would block on the other's.
    #[test]
    fn the_two_channels_lock_separately() {
        assert_ne!(Channel::Slack.lock_key(), Channel::Xyne.lock_key());
    }

    /// The property this whole API is built on.
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

    /// A caller iterating rows must not have to tell a missing key from a null one.
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

    /// The column is `id`, which reads as the row's own identity when it is the announcement's.
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

    /// A forgotten precondition must not read as "apply anyway".
    #[test]
    fn a_write_without_a_precondition_asserts_the_state_was_empty() {
        let request: LifecycleStateWriteRequest =
            serde_json::from_str(r#"{"alerts": []}"#).unwrap();

        assert!(request.expected_last_updated_at.is_none());
        assert!(request.alerts.is_empty());
    }

    /// A field nobody reads is a field the caller thinks is being stored.
    #[test]
    fn an_unknown_field_is_rejected_rather_than_dropped() {
        let error =
            serde_json::from_str::<LifecycleStateWriteRequest>(r#"{"alerts": [], "runs": 4}"#)
                .unwrap_err();

        assert!(error.to_string().contains("runs"));
    }

    /// The `json` columns exist for this.
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

    /// A provider that accepted the message without naming a thread still delivered it, so a null thread id and `sent:
    #[test]
    fn an_announcement_can_be_delivered_without_a_thread() {
        let request: AnnouncementRequest =
            serde_json::from_str(r#"{"sent": true, "ts_slack": null}"#).unwrap();

        assert_eq!(request.sent, Some(true));
        assert!(request.ts_slack.is_none());
    }
}
