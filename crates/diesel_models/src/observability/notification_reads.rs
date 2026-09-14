use diesel::{Identifiable, Insertable, Queryable, Selectable};
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

use crate::observability::schema::notification_reads;

#[derive(
    Clone, Debug, Queryable, Identifiable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = notification_reads, primary_key(user_name), check_for_backend(diesel::pg::Pg))]
pub struct NotificationRead {
    pub user_name: Secret<String>,
    pub last_read_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = notification_reads)]
pub struct NotificationReadNew {
    pub user_name: Secret<String>,
    pub last_read_at: PrimitiveDateTime,
}
