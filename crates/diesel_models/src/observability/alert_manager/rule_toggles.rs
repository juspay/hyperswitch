//! PostgreSQL rows for alert rule enable/disable state.

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_rule_toggles;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_rule_toggles)]
pub struct RuleToggleNew {
    pub rule_id: String,
    pub is_enabled: bool,
    pub updated_by: String,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(
    table_name = alert_rule_toggles,
    primary_key(rule_id),
    check_for_backend(diesel::pg::Pg)
)]
pub struct RuleToggle {
    pub rule_id: String,
    pub is_enabled: bool,
    pub updated_by: String,
    pub last_updated_at: PrimitiveDateTime,
}
