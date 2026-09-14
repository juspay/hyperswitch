use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::{raw_json::RawJson, schema::alerts_main};

#[derive(Clone, Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = alerts_main)]
pub struct AlertsMainNew {
    pub id: uuid::Uuid,
    pub channel: String,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: RawJson,
    pub ts_slack: Option<String>,
    pub ts_alert: PrimitiveDateTime,
    pub duration: i32,
    pub sent: bool,
    pub critical: bool,
    pub rca_metadata: serde_json::Value,
    pub metadata: Option<RawJson>,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = alerts_main, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertsMain {
    pub id: uuid::Uuid,
    pub channel: Option<String>,
    pub name: Option<String>,
    pub product: Option<String>,
    pub dimensions: Option<RawJson>,
    pub ts_slack: Option<String>,
    pub ts_alert: Option<PrimitiveDateTime>,
    pub duration: Option<i32>,
    pub sent: Option<bool>,
    pub critical: Option<bool>,
    pub rca_metadata: Option<serde_json::Value>,
    pub metadata: Option<RawJson>,
    pub last_updated_at: Option<PrimitiveDateTime>,
}
