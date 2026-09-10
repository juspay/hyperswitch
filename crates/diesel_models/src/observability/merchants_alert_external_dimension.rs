use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::merchants_alert_external_dimension;

/// One row of an announcement's per-dimension breakdown.
///
/// No channel twin, and no macro: unlike the instance tables this one exists once and references
/// `alerts_main`, so there is a single struct rather than a generated pair.
// Serialize/Deserialize satisfy `DejaQueryResult`, which the query helpers require under `deja`.
// Insertable as well as Queryable: every column is written, so a separate insert struct would be
// the same fields twice.
#[derive(
    Clone, Debug, PartialEq, Identifiable, Insertable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(
    table_name = merchants_alert_external_dimension,
    primary_key(id_merchant_table),
    check_for_backend(diesel::pg::Pg)
)]
pub struct DimensionInstance {
    pub id: Option<uuid::Uuid>,
    pub id_merchant_table: uuid::Uuid,
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
    pub start_time: Option<PrimitiveDateTime>,
    pub recovered_ts: Option<PrimitiveDateTime>,
    pub ts_slack: Option<String>,
    pub ts_alert: Option<PrimitiveDateTime>,
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub slack_info: Option<serde_json::Value>,
    pub communication_info: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub priority: Option<String>,
    pub tenant_id: Option<String>,
}

impl DimensionInstance {
    pub fn has_recovered(&self) -> bool {
        self.recovered_ts.is_some()
    }
}
