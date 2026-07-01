use common_utils::events::{ApiEventMetric, ApiEventsType};
use serde::{Deserialize, Serialize};

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

/// Per-merchant profile authorisation. Wildcard represents a merchant-level
/// grant (all profiles under the merchant); an explicit list represents
/// profile-level grants.
#[derive(Debug, Clone)]
pub enum ScopeProfiles {
    All,
    Some(Vec<String>),
}

impl Serialize for ScopeProfiles {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::All => s.serialize_str("*"),
            Self::Some(list) => list.serialize(s),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScopeEntry {
    pub merchant_id: String,
    pub profile_ids: ScopeProfiles,
}

/// Outbound mint-request body. Not part of the public API surface —
/// co-located with the response shape so reviewers see the full contract
/// in one place. Identity fields are read from the verified AuthToken
/// inside the handler, never from inbound input.
#[derive(Debug, Clone, Serialize)]
pub struct MintSessionRequest {
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
