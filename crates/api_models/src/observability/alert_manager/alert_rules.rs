//! Typed API models for alert rules.

use common_types::alert_rules::stored::{AlertConfiguration, AlertRuleStatus};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// Body of `PUT /alerts/rules/{id}`: the whole rule, replacing any rule with this id.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleUpsertRequest {
    pub name: String,
    pub status: AlertRuleStatus,
    /// The same JSON as the stored `alerts_configuration`, versions included.
    pub alerts_configuration: AlertConfiguration,
}

/// A stored rule.
#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleResponse {
    pub id: String,
    pub name: String,
    pub status: AlertRuleStatus,
    pub alerts_configuration: AlertConfiguration,
    /// When the evaluator next checks the rule, if the rule is active and rules come from the
    /// database.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub next_run_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleListResponse {
    pub rules: Vec<AlertRuleResponse>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleDeleteResponse {
    pub ok: bool,
    pub id: String,
}
