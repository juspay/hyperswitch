use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::notification_reads;

#[derive(Clone, Debug, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = notification_reads)]
pub struct NotificationReadsNew {
    pub user_name: String,
    pub last_read_at: PrimitiveDateTime,
}

#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = notification_reads, primary_key(user_name), check_for_backend(diesel::pg::Pg))]
pub struct NotificationReads {
    pub user_name: String,
    pub last_read_at: PrimitiveDateTime,
}
