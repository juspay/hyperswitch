//! PostgreSQL rows for alert blacklist state.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_blacklist;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_blacklist)]
pub struct BlacklistEntryNew {
    pub rule_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub reason: String,
    pub created_by: String,
    pub is_deleted: bool,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(
    table_name = alert_blacklist,
    primary_key(rule_id, merchant_id, profile_id),
    check_for_backend(diesel::pg::Pg)
)]
pub struct BlacklistEntry {
    pub rule_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub reason: String,
    pub created_by: String,
    pub last_updated_at: PrimitiveDateTime,
    pub is_deleted: bool,
}

#[derive(Debug)]
pub enum BlacklistUpsertOutcome {
    Stored(BlacklistEntry),
    ActiveRuleLimitReached,
}
