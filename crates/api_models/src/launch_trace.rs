use std::collections::HashMap;

use common_utils::events::{ApiEventMetric, ApiEventsType};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchTraceResponse {
    pub handoff_url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl ApiEventMetric for LaunchTraceResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

/// A single authorisation grant. One `ScopeEntry` per `user_roles` row.
///
/// The shape is deliberately generic so new authorisation dimensions can be
/// added without a schema change:
/// - `entity_type` is a string, not an enum — new levels (e.g. `"connector"`,
///   `"region"`) are additive.
/// - `path` is an open map of ancestor ids — new hierarchy levels just add
///   new keys.
/// - `constraints` is an open JSON bag for attribute-level restrictions
///   (e.g. `{"currency": ["INR"]}`, `{"valid_until": "2026-12-31"}`).
///
/// Consumers deny by default when they encounter an unknown `entity_type` or
/// an unrecognised constraint — old readers stay safe under new shapes.
///
/// Wildcards are implicit at each level: a `merchant`-level grant covers all
/// profiles under that merchant. Only restricted access needs lower-level
/// entries.
#[derive(Debug, Clone, Serialize)]
pub struct ScopeEntry {
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub path: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub constraints: HashMap<String, Value>,
}

/// Outbound Sage mint-request body. Not part of the public API surface —
/// co-located with the response shape so reviewers see the full contract
/// in one place. Identity fields are read from the verified AuthToken
/// inside the handler, never from inbound input.
#[derive(Debug, Clone, Serialize)]
pub struct SageSessionRequest {
    pub user_id: String,
    pub launch_merchant_id: String,
    pub launch_profile_id: String,
    pub org_id: String,
    pub role: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_jti: Option<String>,
    pub scope: Vec<ScopeEntry>,
}
