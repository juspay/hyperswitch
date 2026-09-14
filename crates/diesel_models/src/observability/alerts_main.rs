use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::observability::{raw_json::RawJson, schema::alerts_main};

#[derive(Clone, Debug, Identifiable, Insertable, Queryable, Selectable, Deserialize, Serialize)]
#[diesel(table_name = alerts_main, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AnnouncementRow {
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

impl AnnouncementRow {
    pub fn was_delivered(&self) -> bool {
        self.sent.unwrap_or(false)
    }
}
