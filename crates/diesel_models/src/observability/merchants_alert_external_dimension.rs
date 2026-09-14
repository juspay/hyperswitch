use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::merchants_alert_external_dimension;

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = merchants_alert_external_dimension)]
pub struct MerchantsAlertExternalDimensionNew {
    pub id: uuid::Uuid,
    pub channel: String,
    pub id_merchant_table: uuid::Uuid,
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
    pub start_time: Option<PrimitiveDateTime>,
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: String,
    pub ts_alert: PrimitiveDateTime,
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub last_updated_at: PrimitiveDateTime,
    pub slack_info: serde_json::Value,
    pub communication_info: serde_json::Value,
    pub metadata: serde_json::Value,
    pub metadata_alert_details: serde_json::Value,
    pub priority: String,
    pub tenant_id: String,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = merchants_alert_external_dimension, primary_key(id_merchant_table), check_for_backend(diesel::pg::Pg))]
pub struct MerchantsAlertExternalDimension {
    pub id: uuid::Uuid,
    pub channel: String,
    pub id_merchant_table: uuid::Uuid,
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
    pub start_time: Option<PrimitiveDateTime>,
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: String,
    pub ts_alert: PrimitiveDateTime,
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub last_updated_at: PrimitiveDateTime,
    pub slack_info: serde_json::Value,
    pub communication_info: serde_json::Value,
    pub metadata: serde_json::Value,
    pub metadata_alert_details: serde_json::Value,
    pub priority: String,
    pub tenant_id: String,
}
