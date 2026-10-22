//! PostgreSQL rows for alert rules: what to check, how, and when it is next due.
//!
//! `status` and `alerts_configuration` are read and written as their types from
//! `common_types::observability::alert_rules`.

use common_types::observability::alert_rules::{AlertConfiguration, AlertRuleStatus};
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_rules;

/// A rule to create. `next_run_at` and the timestamps start from their database defaults, so a new
/// rule is due on the evaluator's next tick.
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_rules)]
pub struct AlertRuleNew {
    pub id: String,
    pub name: String,
    pub status: AlertRuleStatus,
    pub alerts_configuration: AlertConfiguration,
}

/// A stored rule.
#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(
    table_name = alert_rules,
    primary_key(id),
    check_for_backend(diesel::pg::Pg)
)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub status: AlertRuleStatus,
    pub alerts_configuration: AlertConfiguration,
    /// When the rule is next due for evaluation (UTC).
    pub next_run_at: PrimitiveDateTime,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}
