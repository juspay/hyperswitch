use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::notification_reads;

// Serialize/Deserialize satisfy `DejaQueryResult`, which the query helpers require under `deja`.
#[derive(
    Clone,
    Debug,
    Queryable,
    Identifiable,
    Insertable,
    Selectable,
    serde::Serialize,
    serde::Deserialize,
)]
#[diesel(table_name = notification_reads, primary_key(user_name), check_for_backend(diesel::pg::Pg))]
pub struct NotificationRead {
    pub user_name: String,
    pub last_read_at: PrimitiveDateTime,
}
