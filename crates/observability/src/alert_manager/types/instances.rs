//! The wire contract for per-merchant alert instances and their per-dimension breakdown:

use diesel_models::observability::{
    merchants_alert_external::MerchantInstanceRow,
    merchants_alert_external_dimension::DimensionInstance,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

/// What a write dropped to stay inside the row cap, and how much.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Truncation {
    /// How many rows the request carried.
    pub received: usize,
    /// How many of them were written.
    pub stored: usize,
    /// How many were not.
    pub dropped: usize,
}

/// One affected merchant, as a write sends it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantInstanceWrite {
    /// The lifecycle row this instance belongs to, if the caller is tracking one.
    #[serde(default)]
    pub id_intermediate: Option<uuid::Uuid>,
    /// The detector that raised it.
    #[serde(default)]
    pub name: Option<String>,
    /// The product it was raised for.
    #[serde(default)]
    pub product: Option<String>,
    /// The merchant this row is about.
    #[serde(default)]
    pub merchant_id: Option<String>,
    /// What the alert is about — profile, connector, whatever the detector split on.
    #[serde(default)]
    pub dimensions: Option<serde_json::Value>,
    /// Anything else identifying the slice, uninterpreted here.
    #[serde(default)]
    pub auxiliary_dimensions: Option<serde_json::Value>,
    /// What was observed.
    #[serde(default)]
    pub current_metric: Option<f64>,
    /// What was expected, or absent when the detector had no expectation.
    #[serde(default)]
    pub expected_metric: Option<f64>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub attribution: Option<String>,
    /// How long the episode has run.
    #[serde(default)]
    pub max_duration: Option<i32>,
    /// When the episode started.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    /// Whether the portal shows the row.
    #[serde(default)]
    pub is_visible: Option<bool>,
    /// When the episode recovered, or absent while it is still firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    /// The provider's thread id.
    #[serde(default)]
    pub ts_slack: Option<String>,
    /// When it was last seen firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub slack_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub communication_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Free-form, and the one field this service adds to:
    #[serde(default)]
    pub metadata_alert_details: Option<serde_json::Value>,
    /// The severity the detector assigned.
    #[serde(default)]
    pub priority: Option<String>,
    /// The tenant the merchant belongs to.
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// One row of the breakdown, as a write sends it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionInstanceWrite {
    /// The lifecycle row this row belongs to, if the caller is tracking one.
    #[serde(default)]
    pub id_intermediate: Option<uuid::Uuid>,
    /// The detector that raised it.
    #[serde(default)]
    pub name: Option<String>,
    /// The product it was raised for.
    #[serde(default)]
    pub product: Option<String>,
    /// What the breakdown is by — `connector`, `payment_method`, and so on.
    #[serde(default)]
    pub dimension_key: Option<String>,
    /// The value of that dimension this row is about.
    #[serde(default)]
    pub dimension_value: Option<String>,
    /// What the alert is about.
    #[serde(default)]
    pub dimensions: Option<serde_json::Value>,
    /// Anything else identifying the slice, uninterpreted here.
    #[serde(default)]
    pub auxiliary_dimensions: Option<serde_json::Value>,
    /// What was observed.
    #[serde(default)]
    pub current_metric: Option<f64>,
    /// What was expected, or absent when the detector had no expectation.
    #[serde(default)]
    pub expected_metric: Option<f64>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub attribution: Option<String>,
    /// How long the episode has run.
    #[serde(default)]
    pub max_duration: Option<i32>,
    /// Whether the portal shows the row.
    #[serde(default)]
    pub is_visible: Option<bool>,
    /// When the episode started.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    /// When the episode recovered, or absent while it is still firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    /// The provider's thread id.
    #[serde(default)]
    pub ts_slack: Option<String>,
    /// When it was last seen firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub slack_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub communication_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    /// Free-form, and where a truncated write records itself.
    #[serde(default)]
    pub metadata_alert_details: Option<serde_json::Value>,
    /// The severity the detector assigned.
    #[serde(default)]
    pub priority: Option<String>,
    /// The tenant the row belongs to.
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// The body of `POST /alerts/instances/{channel}/{announcement_id}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceWriteRequest {
    /// Every affected merchant.
    pub merchants: Vec<MerchantInstanceWrite>,
}

/// The body of `POST /alerts/dimensions/{announcement_id}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionWriteRequest {
    /// Every row of the breakdown.
    pub dimensions: Vec<DimensionInstanceWrite>,
}

/// One affected merchant, as it is stored.
#[derive(Debug, Serialize)]
pub struct MerchantInstanceEntry {
    /// The row's identity, minted by this service.
    pub id_merchant_table: uuid::Uuid,
    /// The announcement this row belongs to.
    pub announcement_id: Option<uuid::Uuid>,
    /// The lifecycle row it belongs to.
    pub id_intermediate: Option<uuid::Uuid>,
    /// The detector that raised it.
    pub name: Option<String>,
    /// The product it was raised for.
    pub product: Option<String>,
    /// The merchant this row is about.
    pub merchant_id: Option<String>,
    /// What the alert is about.
    pub dimensions: Option<serde_json::Value>,
    /// Anything else identifying the slice.
    pub auxiliary_dimensions: Option<serde_json::Value>,
    /// What was observed.
    pub current_metric: Option<f64>,
    /// What was expected.
    pub expected_metric: Option<f64>,
    /// Free-form, uninterpreted here.
    pub attribution: Option<String>,
    /// How long the episode had run.
    pub max_duration: Option<i32>,
    /// When the episode started.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    /// Whether the portal shows the row.
    pub is_visible: Option<bool>,
    /// When the episode recovered.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    /// The provider's thread id, the caller's or the announcement's.
    pub ts_slack: Option<String>,
    /// When this service recorded the row.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    /// When it was last seen firing.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    /// When this service last wrote the row.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    /// Free-form, uninterpreted here.
    pub slack_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    pub communication_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    pub metadata: Option<serde_json::Value>,
    /// Free-form, carrying this service's truncation marker when the write was capped.
    pub metadata_alert_details: Option<serde_json::Value>,
    /// The severity the detector assigned.
    pub priority: Option<String>,
    /// The tenant the merchant belongs to.
    pub tenant_id: Option<String>,
}

/// One row of the breakdown, as it is stored.
#[derive(Debug, Serialize)]
pub struct DimensionInstanceEntry {
    /// The row's identity, minted by this service.
    pub id_merchant_table: uuid::Uuid,
    /// The announcement this row belongs to.
    pub announcement_id: Option<uuid::Uuid>,
    /// The lifecycle row it belongs to.
    pub id_intermediate: Option<uuid::Uuid>,
    /// The detector that raised it.
    pub name: Option<String>,
    /// The product it was raised for.
    pub product: Option<String>,
    /// What the breakdown is by.
    pub dimension_key: Option<String>,
    /// The value of that dimension.
    pub dimension_value: Option<String>,
    /// What the alert is about.
    pub dimensions: Option<serde_json::Value>,
    /// Anything else identifying the slice.
    pub auxiliary_dimensions: Option<serde_json::Value>,
    /// What was observed.
    pub current_metric: Option<f64>,
    /// What was expected.
    pub expected_metric: Option<f64>,
    /// Free-form, uninterpreted here.
    pub attribution: Option<String>,
    /// How long the episode had run.
    pub max_duration: Option<i32>,
    /// Whether the portal shows the row.
    pub is_visible: Option<bool>,
    /// When the episode started.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    /// When the episode recovered.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    /// The provider's thread id.
    pub ts_slack: Option<String>,
    /// When this service recorded the row.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    /// When it was last seen firing.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    /// When this service last wrote the row.
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    /// Free-form, uninterpreted here.
    pub slack_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    pub communication_info: Option<serde_json::Value>,
    /// Free-form, uninterpreted here.
    pub metadata: Option<serde_json::Value>,
    /// Free-form, carrying this service's truncation marker when the write was capped.
    pub metadata_alert_details: Option<serde_json::Value>,
    /// The severity the detector assigned.
    pub priority: Option<String>,
    /// The tenant the row belongs to.
    pub tenant_id: Option<String>,
}

/// What `GET /alerts/instances/{channel}/{announcement_id}` returns.
#[derive(Debug, Serialize)]
pub struct InstanceReadResponse {
    /// Whether the announcement has any instances.
    pub status: ReadStatus,
    /// Every stored instance.
    pub merchants: Vec<MerchantInstanceEntry>,
}

/// What `GET /alerts/dimensions/{announcement_id}` returns.
#[derive(Debug, Serialize)]
pub struct DimensionReadResponse {
    /// Whether the announcement has a breakdown.
    pub status: ReadStatus,
    /// Every stored row of it.
    pub dimensions: Vec<DimensionInstanceEntry>,
}

/// What `POST /alerts/instances/{channel}/{announcement_id}` returns.
#[derive(Debug, Serialize)]
pub struct InstanceSaveResponse {
    /// Always [`WriteStatus::Saved`]; a write that did not apply is an error status.
    pub status: WriteStatus,
    /// The instant every row was stamped with, by this service's clock.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    /// How many rows the announcement now holds.
    pub merchants: usize,
    /// How many rows this write replaced.
    pub removed: usize,
    /// What the row cap dropped, or `null` when the write was stored whole.
    pub truncated: Option<Truncation>,
}

/// What `POST /alerts/dimensions/{announcement_id}` returns.
#[derive(Debug, Serialize)]
pub struct DimensionSaveResponse {
    /// Always [`WriteStatus::Saved`].
    pub status: WriteStatus,
    /// The instant every row was stamped with.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    /// How many rows the breakdown now holds.
    pub dimensions: usize,
    /// How many rows this write replaced.
    pub removed: usize,
    /// What the row cap dropped, or `null` when the breakdown was stored whole.
    pub truncated: Option<Truncation>,
}

impl From<MerchantInstanceRow> for MerchantInstanceEntry {
    fn from(row: MerchantInstanceRow) -> Self {
        Self {
            id_merchant_table: row.id_merchant_table,
            announcement_id: row.id,
            id_intermediate: row.id_intermediate,
            name: row.name,
            product: row.product,
            merchant_id: row.merchant_id,
            dimensions: row.dimensions,
            auxiliary_dimensions: row.auxiliary_dimensions,
            current_metric: row.current_metric,
            expected_metric: row.expected_metric,
            attribution: row.attribution,
            max_duration: row.max_duration,
            start_time: row.start_time,
            is_visible: row.is_visible,
            recovered_ts: row.recovered_ts,
            ts_slack: row.ts_slack,
            ts_alert: row.ts_alert,
            latest_ts_alert: row.latest_ts_alert,
            last_updated_at: row.last_updated_at,
            slack_info: row.slack_info,
            communication_info: row.communication_info,
            metadata: row.metadata,
            metadata_alert_details: row.metadata_alert_details,
            priority: row.priority,
            tenant_id: row.tenant_id,
        }
    }
}

impl From<DimensionInstance> for DimensionInstanceEntry {
    fn from(row: DimensionInstance) -> Self {
        Self {
            id_merchant_table: row.id_merchant_table,
            announcement_id: row.id,
            id_intermediate: row.id_intermediate,
            name: row.name,
            product: row.product,
            dimension_key: row.dimension_key,
            dimension_value: row.dimension_value,
            dimensions: row.dimensions,
            auxiliary_dimensions: row.auxiliary_dimensions,
            current_metric: row.current_metric,
            expected_metric: row.expected_metric,
            attribution: row.attribution,
            max_duration: row.max_duration,
            is_visible: row.is_visible,
            start_time: row.start_time,
            recovered_ts: row.recovered_ts,
            ts_slack: row.ts_slack,
            ts_alert: row.ts_alert,
            latest_ts_alert: row.latest_ts_alert,
            last_updated_at: row.last_updated_at,
            slack_info: row.slack_info,
            communication_info: row.communication_info,
            metadata: row.metadata,
            metadata_alert_details: row.metadata_alert_details,
            priority: row.priority,
            tenant_id: row.tenant_id,
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

    fn bare_row() -> MerchantInstanceRow {
        MerchantInstanceRow {
            id: None,
            id_merchant_table: uuid::Uuid::nil(),
            id_intermediate: None,
            name: None,
            product: None,
            merchant_id: None,
            dimensions: None,
            auxiliary_dimensions: None,
            current_metric: None,
            expected_metric: None,
            attribution: None,
            max_duration: None,
            start_time: None,
            is_visible: None,
            recovered_ts: None,
            ts_slack: None,
            ts_alert: None,
            latest_ts_alert: None,
            last_updated_at: None,
            slack_info: None,
            communication_info: None,
            metadata: None,
            metadata_alert_details: None,
            priority: None,
            tenant_id: None,
        }
    }

    /// A caller iterating rows must not have to tell a missing key from a null one.
    #[test]
    fn an_instance_carries_every_field_even_when_the_columns_are_null() {
        let body = body_of(&MerchantInstanceEntry::from(bare_row()));

        for field in [
            "announcement_id",
            "id_intermediate",
            "name",
            "product",
            "merchant_id",
            "dimensions",
            "auxiliary_dimensions",
            "current_metric",
            "expected_metric",
            "attribution",
            "max_duration",
            "start_time",
            "is_visible",
            "recovered_ts",
            "ts_slack",
            "ts_alert",
            "latest_ts_alert",
            "last_updated_at",
            "slack_info",
            "communication_info",
            "metadata",
            "metadata_alert_details",
            "priority",
            "tenant_id",
        ] {
            assert!(
                body.get(field).is_some_and(serde_json::Value::is_null),
                "{field} was omitted"
            );
        }
    }

    /// The column is `id`, which reads as this row's own identity when it is the announcement's.
    #[test]
    fn the_announcement_reference_is_named_for_what_it_points_at() {
        let announcement = uuid::Uuid::now_v7();
        let mut row = bare_row();
        row.id = Some(announcement);

        let body = body_of(&MerchantInstanceEntry::from(row));

        assert_eq!(body["announcement_id"], announcement.to_string());
        assert_eq!(body["id_merchant_table"], uuid::Uuid::nil().to_string());
        assert!(body.get("id").is_none());
    }

    /// The edge case the ticket names.
    #[test]
    fn an_absent_expected_metric_stays_absent_rather_than_becoming_zero() {
        let write: MerchantInstanceWrite =
            serde_json::from_str(r#"{"merchant_id": "m1", "current_metric": 0.0}"#).unwrap();

        assert_eq!(write.current_metric, Some(0.0));
        assert!(write.expected_metric.is_none());

        let explicit: MerchantInstanceWrite =
            serde_json::from_str(r#"{"expected_metric": null}"#).unwrap();
        assert!(explicit.expected_metric.is_none());
    }

    /// The other half of it:
    #[test]
    fn an_absent_expected_metric_reads_back_as_null() {
        let mut row = bare_row();
        row.current_metric = Some(0.0);

        let body = body_of(&MerchantInstanceEntry::from(row));

        assert_eq!(body["current_metric"], 0.0);
        assert!(body["expected_metric"].is_null());
    }

    /// A field nobody stores is a field the caller believes is being stored.
    #[test]
    fn an_unknown_field_is_rejected_rather_than_dropped() {
        let error = serde_json::from_str::<InstanceWriteRequest>(
            r#"{"merchants": [{"merchant_id": "m1", "sr": 41.5}]}"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("sr"));
    }

    /// The breakdown has no merchant column, so a body naming one is a caller that has confused the two routes and should hear about it here rather than have the field dropped.
    #[test]
    fn a_dimension_row_cannot_name_a_merchant() {
        assert!(serde_json::from_str::<DimensionWriteRequest>(
            r#"{"dimensions": [{"merchant_id": "m1"}]}"#
        )
        .is_err());
    }

    /// A write that was stored whole says so by carrying no truncation at all, rather than by carrying zeroes a reader has to interpret.
    #[test]
    fn a_write_that_was_not_capped_reports_no_truncation() {
        let body = body_of(&InstanceSaveResponse {
            status: WriteStatus::Saved,
            ts_alert: common_utils::date_time::now(),
            merchants: 3,
            removed: 0,
            truncated: None,
        });

        assert_eq!(body["merchants"], 3);
        assert!(body["truncated"].is_null());
    }

    /// And one that was capped says what it dropped, in the response as well as in the rows.
    #[test]
    fn a_capped_write_reports_what_it_dropped() {
        let body = body_of(&DimensionSaveResponse {
            status: WriteStatus::Saved,
            ts_alert: common_utils::date_time::now(),
            dimensions: 500,
            removed: 0,
            truncated: Some(Truncation {
                received: 1_240,
                stored: 500,
                dropped: 740,
            }),
        });

        assert_eq!(body["dimensions"], 500);
        assert_eq!(body["truncated"]["received"], 1_240);
        assert_eq!(body["truncated"]["stored"], 500);
        assert_eq!(body["truncated"]["dropped"], 740);
    }
}
