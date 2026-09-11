use diesel_models::observability::{
    merchants_alert_external::MerchantInstanceRow,
    merchants_alert_external_dimension::DimensionInstance,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Truncation {
    pub received: usize,
    pub stored: usize,
    pub dropped: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantInstanceWrite {
    #[serde(default)]
    pub id_intermediate: Option<uuid::Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub merchant_id: Option<String>,
    #[serde(default)]
    pub dimensions: Option<serde_json::Value>,
    #[serde(default)]
    pub auxiliary_dimensions: Option<serde_json::Value>,
    #[serde(default)]
    pub current_metric: Option<f64>,
    #[serde(default)]
    pub expected_metric: Option<f64>,
    #[serde(default)]
    pub attribution: Option<String>,
    #[serde(default)]
    pub max_duration: Option<i32>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    #[serde(default)]
    pub is_visible: Option<bool>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    #[serde(default)]
    pub ts_slack: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    #[serde(default)]
    pub slack_info: Option<serde_json::Value>,
    #[serde(default)]
    pub communication_info: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata_alert_details: Option<serde_json::Value>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionInstanceWrite {
    #[serde(default)]
    pub id_intermediate: Option<uuid::Uuid>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub product: Option<String>,
    #[serde(default)]
    pub dimension_key: Option<String>,
    #[serde(default)]
    pub dimension_value: Option<String>,
    #[serde(default)]
    pub dimensions: Option<serde_json::Value>,
    #[serde(default)]
    pub auxiliary_dimensions: Option<serde_json::Value>,
    #[serde(default)]
    pub current_metric: Option<f64>,
    #[serde(default)]
    pub expected_metric: Option<f64>,
    #[serde(default)]
    pub attribution: Option<String>,
    #[serde(default)]
    pub max_duration: Option<i32>,
    #[serde(default)]
    pub is_visible: Option<bool>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    #[serde(default)]
    pub ts_slack: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    #[serde(default)]
    pub slack_info: Option<serde_json::Value>,
    #[serde(default)]
    pub communication_info: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata_alert_details: Option<serde_json::Value>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceWriteRequest {
    pub merchants: Vec<MerchantInstanceWrite>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionWriteRequest {
    pub dimensions: Vec<DimensionInstanceWrite>,
}

#[derive(Debug, Serialize)]
pub struct MerchantInstanceEntry {
    pub id_merchant_table: uuid::Uuid,
    pub announcement_id: Option<uuid::Uuid>,
    pub id_intermediate: Option<uuid::Uuid>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub merchant_id: Option<String>,
    pub dimensions: Option<serde_json::Value>,
    pub auxiliary_dimensions: Option<serde_json::Value>,
    pub current_metric: Option<f64>,
    pub expected_metric: Option<f64>,
    pub attribution: Option<String>,
    pub max_duration: Option<i32>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    pub is_visible: Option<bool>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub slack_info: Option<serde_json::Value>,
    pub communication_info: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub priority: Option<String>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DimensionInstanceEntry {
    pub id_merchant_table: uuid::Uuid,
    pub announcement_id: Option<uuid::Uuid>,
    pub id_intermediate: Option<uuid::Uuid>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimension_key: Option<String>,
    pub dimension_value: Option<String>,
    pub dimensions: Option<serde_json::Value>,
    pub auxiliary_dimensions: Option<serde_json::Value>,
    pub current_metric: Option<f64>,
    pub expected_metric: Option<f64>,
    pub attribution: Option<String>,
    pub max_duration: Option<i32>,
    pub is_visible: Option<bool>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub slack_info: Option<serde_json::Value>,
    pub communication_info: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub priority: Option<String>,
    pub tenant_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InstanceReadResponse {
    pub status: ReadStatus,
    pub merchants: Vec<MerchantInstanceEntry>,
}

#[derive(Debug, Serialize)]
pub struct DimensionReadResponse {
    pub status: ReadStatus,
    pub dimensions: Vec<DimensionInstanceEntry>,
}

#[derive(Debug, Serialize)]
pub struct InstanceSaveResponse {
    pub status: WriteStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    pub merchants: usize,
    pub removed: usize,
    pub truncated: Option<Truncation>,
}

#[derive(Debug, Serialize)]
pub struct DimensionSaveResponse {
    pub status: WriteStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    pub dimensions: usize,
    pub removed: usize,
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

    #[test]
    fn an_absent_expected_metric_reads_back_as_null() {
        let mut row = bare_row();
        row.current_metric = Some(0.0);

        let body = body_of(&MerchantInstanceEntry::from(row));

        assert_eq!(body["current_metric"], 0.0);
        assert!(body["expected_metric"].is_null());
    }

    #[test]
    fn an_unknown_field_is_rejected_rather_than_dropped() {
        let error = serde_json::from_str::<InstanceWriteRequest>(
            r#"{"merchants": [{"merchant_id": "m1", "sr": 41.5}]}"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("sr"));
    }

    #[test]
    fn a_dimension_row_cannot_name_a_merchant() {
        assert!(serde_json::from_str::<DimensionWriteRequest>(
            r#"{"dimensions": [{"merchant_id": "m1"}]}"#
        )
        .is_err());
    }

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
