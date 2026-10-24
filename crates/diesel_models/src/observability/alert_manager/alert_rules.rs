//! PostgreSQL rows for alert rules: what to check, how, and when it is next due.
//!
//! `status` and `alerts_configuration` are read and written as their types from
//! `common_types::alert_rules::stored`.

use common_types::alert_rules::stored::{AlertConfiguration, AlertRuleStatus};
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_rules;

/// A rule to create or replace. The timestamps start from their database defaults.
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_rules)]
pub struct AlertRuleNew {
    pub id: String,
    pub name: String,
    pub status: AlertRuleStatus,
    pub alerts_configuration: AlertConfiguration,
    /// Set from the service's UTC clock, not the database default: the column has no time zone,
    /// and the evaluator compares it with its own clock.
    pub next_run_at: PrimitiveDateTime,
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
