//! PostgreSQL rows for alert lifecycle episode state.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_lifecycle_events;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_lifecycle_events)]
pub struct LifecycleEventNew {
    pub alert_key: String,
    pub detector: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub state: String,
    pub first_seen: PrimitiveDateTime,
    pub last_seen: PrimitiveDateTime,
    pub recovered_at: PrimitiveDateTime,
    pub runs: i64,
    pub severity: String,
    pub sr: f64,
    pub failed: i64,
    pub total: i64,
    pub connector: String,
    pub notified_at: PrimitiveDateTime,
    pub ts_slack: String,
    pub sent: bool,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = alert_lifecycle_events, primary_key(alert_key), check_for_backend(diesel::pg::Pg))]
pub struct LifecycleEvent {
    pub alert_key: String,
    pub detector: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub state: String,
    pub first_seen: PrimitiveDateTime,
    pub last_seen: PrimitiveDateTime,
    pub recovered_at: PrimitiveDateTime,
    pub runs: i64,
    pub severity: String,
    pub sr: f64,
    pub failed: i64,
    pub total: i64,
    pub connector: String,
    pub notified_at: PrimitiveDateTime,
    pub ts_slack: String,
    pub sent: bool,
    pub last_updated_at: PrimitiveDateTime,
}
