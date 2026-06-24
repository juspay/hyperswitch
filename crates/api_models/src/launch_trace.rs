//! Request / response types for `POST /user/launch_trace`.
//!
//! Mints a federated HyperSage Trace session for a Control Center user
//! who clicked the Trace launcher. The HS BE handler reads the user's
//! identity from the verified `AuthToken` (NEVER from the request body)
//! and proxies a mint request to HyperSage's
//! `POST /api/sage/session` endpoint with a shared infra-key Bearer.
//!
//! The HyperSage side is documented in
//! <https://github.com/juspay/hypersage/blob/main/docs/SAGE_SESSION_API.md>.
//!
//! Tracking: juspay/hypersage#1040 (parent), #1066 (spike), #1067 (impl).

use common_utils::events::{ApiEventMetric, ApiEventsType};
use serde::{Deserialize, Serialize};

/// The response returned to the CC frontend. The browser is expected
/// to redirect (or pop a new tab) to `handoff_url`; HyperSage's
/// `GET /handoff` endpoint will swap the URL-carried token for an
/// `HttpOnly` cookie and 302 to `/trace`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchTraceResponse {
    /// Absolute URL the browser navigates to. Single-use; the embedded
    /// token dies on first redeem (60s TTL on the HyperSage side).
    pub handoff_url: String,

    /// ISO-8601 UTC instant after which the handoff URL is dead.
    /// Optional during the dark-launch window because the HyperSage
    /// response may or may not include it; surfaced to the client only
    /// when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

impl ApiEventMetric for LaunchTraceResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        // `Miscellaneous` matches what `get_active_user_details`
        // emits — federation isn't payment-scoped and the
        // response carries no domain ids the event-bus would index.
        Some(ApiEventsType::Miscellaneous)
    }
}

/// Outbound request body to HyperSage `POST /api/sage/session`. NOT
/// part of the HS BE public API surface — declared here only so the
/// shape is co-located with the response type for one-PR review.
///
/// HyperSage's auth_users + merchant_ids allowlist is the authoritative
/// merchant-access barrier (see
/// `hypersage/docs/SAGE_SESSION_API.md` §2). HS BE's responsibility
/// is identity attestation only — pass through the claims we extracted
/// from the verified AuthToken.
#[derive(Debug, Clone, Serialize)]
pub struct SageSessionRequest {
    pub user_id: String,
    pub merchant_id: String,
    pub profile_id: String,
    pub org_id: String,
    pub role: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_jti: Option<String>,
}
