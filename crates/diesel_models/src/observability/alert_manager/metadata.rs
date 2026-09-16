//! PostgreSQL rows for per-alert metadata and snooze state.

use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_metadata;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_metadata)]
pub struct AlertMetadataNew {
    pub id: String,
    pub metadata: String,
    pub snooze: String,
    pub updated_by: String,
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alert_metadata)]
pub struct AlertMetadataChangeset {
    pub metadata: Option<String>,
    pub snooze: Option<String>,
    pub updated_by: String,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = alert_metadata, check_for_backend(diesel::pg::Pg))]
pub struct AlertMetadataEntry {
    pub id: String,
    pub metadata: String,
    pub snooze: String,
    pub updated_by: String,
    pub last_updated_at: PrimitiveDateTime,
}
