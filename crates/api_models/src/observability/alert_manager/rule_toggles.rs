//! Typed API models for alert rule enable/disable state.

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// Replacement body for one rule toggle.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleToggleSetRequest {
    pub is_enabled: bool,
    pub updated_by: String,
}

/// A stored rule toggle.
#[derive(Clone, Debug, Serialize)]
pub struct RuleToggleResponse {
    pub rule_id: String,
    pub is_enabled: bool,
    pub updated_by: String,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleToggleListResponse {
    pub toggles: Vec<RuleToggleResponse>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleToggleSetResponse {
    pub ok: bool,
    pub id: String,
    pub is_enabled: bool,
}
