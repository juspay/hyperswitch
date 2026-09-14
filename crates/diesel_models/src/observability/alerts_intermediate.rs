use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::alerts_intermediate;

#[derive(
    Clone, Debug, PartialEq, Identifiable, Insertable, Queryable, Selectable, Deserialize, Serialize,
)]
#[diesel(table_name = alerts_intermediate, primary_key(id_intermediate))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AlertStateRow {
    pub id_intermediate: uuid::Uuid,
    pub channel: Option<String>,
    pub id: Option<uuid::Uuid>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: Option<serde_json::Value>,
    pub ts_slack: Option<String>,
    pub ts_alert: Option<PrimitiveDateTime>,
    pub latest_ts_alert: Option<PrimitiveDateTime>,
    pub max_duration: Option<i32>,
    pub other_metrics: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub rca_metadata: Option<serde_json::Value>,
    pub group_id: Option<String>,
    pub priority: Option<String>,
    pub last_updated_at: Option<PrimitiveDateTime>,
    pub recovered_ts: Option<PrimitiveDateTime>,
}

impl AlertStateRow {
    pub fn has_recovered(&self) -> bool {
        self.recovered_ts.is_some()
    }
}
