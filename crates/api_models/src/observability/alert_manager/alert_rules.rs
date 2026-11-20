//! Typed API models for alert rules.

use common_types::alert_rules::{stored::AlertRuleStatus, AlertRuleConfig};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// Body of `POST /alerts/rules`: a new rule and the id it is created under.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleCreateRequest {
    pub id: String,
    pub name: String,
    pub status: AlertRuleStatus,
    /// As in [`AlertRuleUpdateRequest::alerts_configuration`].
    pub alerts_configuration: AlertRuleConfig,
}

/// Body of `PUT /alerts/rules/{id}`: the whole rule, replacing the existing rule with this id.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlertRuleUpdateRequest {
    pub name: String,
    pub status: AlertRuleStatus,
    /// How the rule is checked, in the evaluator's shape: the same JSON as a config rule's
    /// `config`. It is stored with versions; they are not part of the API.
    pub alerts_configuration: AlertRuleConfig,
}

/// A stored rule.
#[derive(Clone, Debug, Serialize)]
pub struct AlertRuleResponse {
    pub id: String,
    pub name: String,
    pub status: AlertRuleStatus,
    pub alerts_configuration: AlertRuleConfig,
    /// When the evaluator next checks the rule, if the rule is active and rules come from the
    /// database.
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub next_run_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: PrimitiveDateTime,
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
