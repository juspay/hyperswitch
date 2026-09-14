use diesel_models::observability::{
    alerts_main::AlertsMain,
    merchants_alert_external::{MerchantsAlertExternal, MerchantsAlertExternalNew},
    merchants_alert_external_dimension::{
        MerchantsAlertExternalDimension, MerchantsAlertExternalDimensionNew,
    },
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use super::within_width;
use crate::errors::ObservabilityApiResult;

const NAME_MAX_CHARS: usize = 64;

const VALUE_MAX_CHARS: usize = 255;

const EMPTY_DOCUMENT: &str = "{}";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantInstanceWrite {
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
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    pub is_visible: Option<bool>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub slack_info: Option<serde_json::Value>,
    pub communication_info: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub priority: Option<String>,
    pub tenant_id: Option<String>,
}

impl MerchantInstanceWrite {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        within_width("name", self.name.as_deref(), NAME_MAX_CHARS)?;
        within_width("product", self.product.as_deref(), NAME_MAX_CHARS)?;
        within_width("merchant_id", self.merchant_id.as_deref(), NAME_MAX_CHARS)?;
        within_width("priority", self.priority.as_deref(), NAME_MAX_CHARS)?;
        within_width("tenant_id", self.tenant_id.as_deref(), NAME_MAX_CHARS)?;
        within_width("attribution", self.attribution.as_deref(), VALUE_MAX_CHARS)?;
        within_width("ts_slack", self.ts_slack.as_deref(), VALUE_MAX_CHARS)
    }

    pub fn into_insertable(
        self,
        id_merchant_table: uuid::Uuid,
        announcement: &AlertsMain,
        now: PrimitiveDateTime,
    ) -> MerchantsAlertExternalNew {
        MerchantsAlertExternalNew {
            id: announcement.id,
            channel: announcement.channel.clone(),
            id_merchant_table,
            id_intermediate: self.id_intermediate,
            name: self.name.unwrap_or_default(),
            product: self.product.unwrap_or_default(),
            merchant_id: self.merchant_id.unwrap_or_default(),
            dimensions: self.dimensions.unwrap_or_else(empty_document_text),
            auxiliary_dimensions: self
                .auxiliary_dimensions
                .unwrap_or_else(empty_document_text),
            current_metric: self.current_metric,
            expected_metric: self.expected_metric,
            attribution: self.attribution.unwrap_or_default(),
            max_duration: self.max_duration,
            start_time: self.start_time,
            is_visible: self.is_visible.unwrap_or(true),
            recovered_ts: self.recovered_ts,
            ts_slack: self
                .ts_slack
                .or_else(|| announcement.ts_slack.clone())
                .unwrap_or_default(),
            ts_alert: now,
            latest_ts_alert: self.latest_ts_alert,
            last_updated_at: now,
            slack_info: self.slack_info.unwrap_or_else(empty_object),
            communication_info: self.communication_info.unwrap_or_else(empty_object),
            metadata: self.metadata.unwrap_or_else(empty_document_text),
            metadata_alert_details: self.metadata_alert_details.unwrap_or_else(empty_object),
            priority: self.priority.unwrap_or_default(),
            tenant_id: self.tenant_id.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionInstanceWrite {
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
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: Option<String>,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub slack_info: Option<serde_json::Value>,
    pub communication_info: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub priority: Option<String>,
    pub tenant_id: Option<String>,
}

impl DimensionInstanceWrite {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        within_width("name", self.name.as_deref(), NAME_MAX_CHARS)?;
        within_width("product", self.product.as_deref(), NAME_MAX_CHARS)?;
        within_width(
            "dimension_key",
            self.dimension_key.as_deref(),
            NAME_MAX_CHARS,
        )?;
        within_width("priority", self.priority.as_deref(), NAME_MAX_CHARS)?;
        within_width("tenant_id", self.tenant_id.as_deref(), NAME_MAX_CHARS)?;
        within_width(
            "dimension_value",
            self.dimension_value.as_deref(),
            VALUE_MAX_CHARS,
        )?;
        within_width("attribution", self.attribution.as_deref(), VALUE_MAX_CHARS)?;
        within_width("ts_slack", self.ts_slack.as_deref(), VALUE_MAX_CHARS)
    }

    pub fn into_insertable(
        self,
        id_merchant_table: uuid::Uuid,
        announcement: &AlertsMain,
        now: PrimitiveDateTime,
    ) -> MerchantsAlertExternalDimensionNew {
        MerchantsAlertExternalDimensionNew {
            id: announcement.id,
            channel: announcement.channel.clone(),
            id_merchant_table,
            id_intermediate: self.id_intermediate,
            name: self.name.unwrap_or_default(),
            product: self.product.unwrap_or_default(),
            dimension_key: self.dimension_key.unwrap_or_default(),
            dimension_value: self.dimension_value.unwrap_or_default(),
            dimensions: self.dimensions.unwrap_or_else(empty_document_text),
            auxiliary_dimensions: self
                .auxiliary_dimensions
                .unwrap_or_else(empty_document_text),
            current_metric: self.current_metric,
            expected_metric: self.expected_metric,
            attribution: self.attribution.unwrap_or_default(),
            max_duration: self.max_duration,
            is_visible: self.is_visible.unwrap_or(true),
            start_time: self.start_time,
            recovered_ts: self.recovered_ts,
            ts_slack: self
                .ts_slack
                .or_else(|| announcement.ts_slack.clone())
                .unwrap_or_default(),
            ts_alert: now,
            latest_ts_alert: self.latest_ts_alert,
            last_updated_at: now,
            slack_info: self.slack_info.unwrap_or_else(empty_object),
            communication_info: self.communication_info.unwrap_or_else(empty_object),
            metadata: self.metadata.unwrap_or_else(empty_document_text),
            metadata_alert_details: self.metadata_alert_details.unwrap_or_else(empty_object),
            priority: self.priority.unwrap_or_default(),
            tenant_id: self.tenant_id.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceWriteRequest {
    pub merchants: Vec<MerchantInstanceWrite>,
}

impl InstanceWriteRequest {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        self.merchants
            .iter()
            .try_for_each(MerchantInstanceWrite::validate)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionWriteRequest {
    pub dimensions: Vec<DimensionInstanceWrite>,
}

impl DimensionWriteRequest {
    pub fn validate(&self) -> ObservabilityApiResult<()> {
        self.dimensions
            .iter()
            .try_for_each(DimensionInstanceWrite::validate)
    }
}

#[derive(Debug, Serialize)]
pub struct MerchantInstanceEntry {
    pub id_merchant_table: uuid::Uuid,
    pub announcement_id: uuid::Uuid,
    pub id_intermediate: Option<uuid::Uuid>,
    pub name: String,
    pub product: String,
    pub merchant_id: String,
    pub dimensions: serde_json::Value,
    pub auxiliary_dimensions: serde_json::Value,
    pub current_metric: Option<f64>,
    pub expected_metric: Option<f64>,
    pub attribution: String,
    pub max_duration: Option<i32>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    pub is_visible: bool,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
    pub slack_info: serde_json::Value,
    pub communication_info: serde_json::Value,
    pub metadata: serde_json::Value,
    pub metadata_alert_details: serde_json::Value,
    pub priority: String,
    pub tenant_id: String,
}

#[derive(Debug, Serialize)]
pub struct DimensionInstanceEntry {
    pub id_merchant_table: uuid::Uuid,
    pub announcement_id: uuid::Uuid,
    pub id_intermediate: Option<uuid::Uuid>,
    pub name: String,
    pub product: String,
    pub dimension_key: String,
    pub dimension_value: String,
    pub dimensions: serde_json::Value,
    pub auxiliary_dimensions: serde_json::Value,
    pub current_metric: Option<f64>,
    pub expected_metric: Option<f64>,
    pub attribution: String,
    pub max_duration: Option<i32>,
    pub is_visible: bool,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_time: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub ts_alert: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
    pub slack_info: serde_json::Value,
    pub communication_info: serde_json::Value,
    pub metadata: serde_json::Value,
    pub metadata_alert_details: serde_json::Value,
    pub priority: String,
    pub tenant_id: String,
}

#[derive(Debug, Serialize)]
pub struct InstanceListResponse {
    pub count: usize,
    pub merchants: Vec<MerchantInstanceEntry>,
}

#[derive(Debug, Serialize)]
pub struct DimensionListResponse {
    pub count: usize,
    pub dimensions: Vec<DimensionInstanceEntry>,
}

#[derive(Debug, Serialize)]
pub struct InstanceSaveResponse {
    pub stored: usize,
    pub removed: usize,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub ts_alert: Option<PrimitiveDateTime>,
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

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

fn empty_document_text() -> serde_json::Value {
    serde_json::Value::String(EMPTY_DOCUMENT.to_owned())
}
