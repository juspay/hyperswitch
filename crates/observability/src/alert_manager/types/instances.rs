//! The wire contract for per-merchant alert instances and their per-dimension breakdown.
//!
//! An announcement says *an alert went out*. These rows say **who it was about**: one row per
//! affected merchant, and one row per dimension of the breakdown behind it — connector, payment
//! method, whatever the detector split on.
//!
//! ## Both resources hang off an announcement
//!
//! The announcement is a path segment, never a body field. Both tables reference `alerts_main`
//! `ON DELETE CASCADE`, so the relationship is the schema's and this API keeps it rather than
//! leaving every caller to fill an `id` column correctly: a caller records the announcement, gets
//! an id back, and posts its instances to that id. There is no way to write an instance that
//! points at nothing, and no way to write one that points at an announcement that does not exist.
//!
//! ## Why the breakdown is not channel-twinned
//!
//! `merchants_alert_external` and `merchants_alert_external_xyne` are the same table once per
//! delivery channel, and are addressed as `/alerts/instances/{channel}/{announcement}`.
//! `merchants_alert_external_dimension` exists **once** and references `alerts_main`, so it is
//! addressed as `/alerts/dimensions/{announcement}` with no channel in the path. Putting a channel
//! there would promise a `_xyne` breakdown table that does not exist, and answering "none" for it
//! would read as "this alert had no breakdown".
//!
//! ## What the caller sends and what the server owns
//!
//! None of these columns carries a `DEFAULT` any more — see the migration — so every value one
//! used to supply is supplied here instead:
//!
//! | Column | Who fills it |
//! |---|---|
//! | `id` | the path — the announcement these rows belong to |
//! | `id_merchant_table` | the server, `uuid::Uuid::now_v7()`; it had `gen_random_uuid()` |
//! | `ts_alert` | the server's clock; it had `CURRENT_TIMESTAMP` |
//! | `last_updated_at` | the server's clock |
//! | `is_visible` | the caller, or `true` when absent; it had `DEFAULT TRUE` |
//! | `ts_slack` | the caller, or the announcement's thread when absent |
//! | everything else | the caller |
//!
//! ## Absent is not zero
//!
//! `current_metric` and `expected_metric` are `Option<f64>` and an absent one stays absent. The
//! alert manager's absolutes — zero volume, zero success — have no expected value at all, and
//! writing `0` for them would store "observed 0, expected 0", which reads as healthy. They are
//! also what the truncation order is computed from, where the same rule applies: see
//! [`super::super::core::instances`].

use diesel_models::observability::{
    merchants_alert_external::MerchantInstanceRow,
    merchants_alert_external_dimension::DimensionInstance,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

/// What a write dropped to stay inside the row cap, and how much.
///
/// Present on a response only when something was dropped, and recorded on every row the write
/// kept — see [`super::super::core::instances`] for where it lands. A breakdown that silently
/// arrived shortened would make an outage look narrower than it was, which is the one reading
/// these rows exist to prevent.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Truncation {
    /// How many rows the request carried.
    pub received: usize,
    /// How many of them were written.
    pub stored: usize,
    /// How many were not. `received - stored`, spelled out so a reader does not have to subtract.
    pub dropped: usize,
}

/// One affected merchant, as a write sends it.
///
/// Every field is optional because every column is nullable, and because a detector reports what
/// it has: an absolute has no `expected_metric`, and an instance recorded before its announcement
/// reached a channel has no `ts_slack`.
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
    ///
    /// Absent stays absent. It is never coerced to `0`, which is a real reading — a zero-volume
    /// alert observes exactly that.
    #[serde(default)]
    pub current_metric: Option<f64>,
    /// What was expected, or absent when the detector had no expectation.
    ///
    /// The absolutes send it absent: "zero payments succeeded" is not measured against anything.
    /// Storing `0` here instead would read as "observed 0, expected 0" — a healthy row.
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
    /// Whether the portal shows the row. `true` when absent — the column lost its `DEFAULT TRUE`,
    /// and a row nobody can see is not what a caller that said nothing meant.
    #[serde(default)]
    pub is_visible: Option<bool>,
    /// When the episode recovered, or absent while it is still firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    /// The provider's thread id.
    ///
    /// Absent takes the announcement's, so a caller does not carry the thread around; absent on
    /// both is stored as `null`, which is what an instance recorded before its announcement
    /// reached a channel honestly has.
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
    /// Free-form, and the one field this service adds to: a truncated write records itself here.
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
///
/// The instance shape with the merchant swapped for the dimension it is broken down by. Kept as
/// its own type rather than one type with three optional discriminators, so that a request cannot
/// name a merchant on a route that has no column for one.
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
    /// What was observed. Absent stays absent — see [`MerchantInstanceWrite::current_metric`].
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
    /// Whether the portal shows the row. `true` when absent.
    #[serde(default)]
    pub is_visible: Option<bool>,
    /// When the episode started.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    /// When the episode recovered, or absent while it is still firing.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    /// The provider's thread id. Absent takes the announcement's.
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
///
/// The whole of one announcement's instances. A write replaces what that announcement already has,
/// which makes a retried run idempotent rather than doubling every merchant.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceWriteRequest {
    /// Every affected merchant. An empty list is a legitimate write: it clears the announcement's
    /// instances.
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
    ///
    /// Named for what it points at rather than for its column, which is `id` and would read as
    /// this row's own identity beside `id_merchant_table`.
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
    /// What was observed. `null` when the detector reported none.
    pub current_metric: Option<f64>,
    /// What was expected. `null` for an absolute, which expected nothing.
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
    /// When the episode recovered. `null` means it is still firing.
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
    /// What was observed. `null` when the detector reported none.
    pub current_metric: Option<f64>,
    /// What was expected. `null` for an absolute.
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
    /// When the episode recovered. `null` means it is still firing.
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
    /// Whether the announcement has any instances. An announcement that exists and affected
    /// nobody is [`ReadStatus::Absent`] and a `200`; a store that could not be read is a `503`.
    pub status: ReadStatus,
    /// Every stored instance. Never `null`.
    pub merchants: Vec<MerchantInstanceEntry>,
}

/// What `GET /alerts/dimensions/{announcement_id}` returns.
#[derive(Debug, Serialize)]
pub struct DimensionReadResponse {
    /// Whether the announcement has a breakdown.
    pub status: ReadStatus,
    /// Every stored row of it. Never `null`.
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
    /// How many rows this write replaced. A write is a replacement, so a retried run reports what
    /// its predecessor left rather than doubling the table.
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

    /// The edge case the ticket names. "Observed 0, expected 0" reads as healthy, so an absolute —
    /// which expects nothing — must be able to say so rather than being made to supply a number.
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

    /// The other half of it: a stored absence comes back as `null`, not as `0`.
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

    /// The breakdown has no merchant column, so a body naming one is a caller that has confused
    /// the two routes and should hear about it here rather than have the field dropped.
    #[test]
    fn a_dimension_row_cannot_name_a_merchant() {
        assert!(serde_json::from_str::<DimensionWriteRequest>(
            r#"{"dimensions": [{"merchant_id": "m1"}]}"#
        )
        .is_err());
    }

    /// A write that was stored whole says so by carrying no truncation at all, rather than by
    /// carrying zeroes a reader has to interpret.
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
