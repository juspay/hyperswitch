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

/// Outbound mint-request body. Not part of the public API surface —
/// co-located with the response shape so reviewers see the full contract
/// in one place. Identity fields are read from the verified AuthToken
/// inside the handler, never from inbound input.
#[derive(Debug, Clone, Serialize)]
pub struct MintSessionRequest {
    pub user_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub org_id: String,
    pub role: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_jti: Option<String>,
}
