use diesel_models::observability::{
    merchants_alert_external::MerchantsAlertExternal,
    merchants_alert_external_dimension::MerchantsAlertExternalDimension,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::{ReadStatus, WriteStatus};

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
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    pub merchants: usize,
    pub removed: usize,
}

#[derive(Debug, Serialize)]
pub struct DimensionSaveResponse {
    pub status: WriteStatus,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
    pub dimensions: usize,
    pub removed: usize,
}

impl From<MerchantsAlertExternal> for MerchantInstanceEntry {
    fn from(row: MerchantsAlertExternal) -> Self {
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

impl From<MerchantsAlertExternalDimension> for DimensionInstanceEntry {
    fn from(row: MerchantsAlertExternalDimension) -> Self {
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
