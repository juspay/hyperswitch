use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::schema::alerts_intermediate;

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = alerts_intermediate)]
pub struct AlertsIntermediateNew {
    pub id_intermediate: uuid::Uuid,
    pub channel: String,
    pub id: Option<uuid::Uuid>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: serde_json::Value,
    pub ts_slack: Option<String>,
    pub ts_alert: PrimitiveDateTime,
    pub latest_ts_alert: PrimitiveDateTime,
    pub max_duration: i32,
    pub other_metrics: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub metadata_alert_details: Option<serde_json::Value>,
    pub rca_metadata: serde_json::Value,
    pub group_id: String,
    pub priority: String,
    pub last_updated_at: PrimitiveDateTime,
    pub recovered_ts: Option<PrimitiveDateTime>,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = alerts_intermediate, primary_key(id_intermediate), check_for_backend(diesel::pg::Pg))]
pub struct AlertsIntermediate {
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

#[derive(Debug)]
pub enum AlertsIntermediateUpdate {
    Metadata { metadata: serde_json::Value },
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alerts_intermediate)]
pub struct AlertsIntermediateUpdateInternal {
    pub metadata: Option<serde_json::Value>,
}

impl From<AlertsIntermediateUpdate> for AlertsIntermediateUpdateInternal {
    fn from(update: AlertsIntermediateUpdate) -> Self {
        match update {
            AlertsIntermediateUpdate::Metadata { metadata } => Self {
                metadata: Some(metadata),
            },
        }
    }
}
