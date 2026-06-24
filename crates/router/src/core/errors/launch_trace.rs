//! Error enum for the federated Trace session mint handler.
//!
//! The error → HTTP status mapping intentionally diverges from the
//! upstream HyperSage status in a few places — see the
//! `LaunchTraceErrors::switch` impl for the rationale, and the
//! "Sage 401 → HS 502" note in the parent PR description (a CC user
//! whose JWT is valid should NEVER be 401'd by HS BE just because the
//! shared infra key is misconfigured; that's a server-side bug, not
//! a client auth failure).
//!
//! Mirrors `crates/router/src/core/errors/chat.rs`.
//!
//! Tracking: juspay/hypersage#1040.

#[derive(Debug, thiserror::Error)]
pub enum LaunchTraceErrors {
    /// The feature flag is off in this env. Returned as **404** to
    /// match HyperSage's own no-oracle policy
    /// (`docs/SAGE_SESSION_API.md` §3) — neither side reveals whether
    /// federation is even available without proof of identity.
    #[error("Federated Trace sessions are not enabled in this environment")]
    FeatureDisabled,

    /// HyperSage said the role / merchant binding can't be granted.
    /// E.g., internal role attempting federation, or HS BE constructed
    /// a request HyperSage refused (422). We do NOT bubble the upstream
    /// detail to the client — it could leak the bound merchant.
    #[error("Federated Trace is not available for this role")]
    Forbidden,

    /// HyperSage's session store is unavailable (Redis down on their
    /// side, or HyperSage's PG audit write failed in a way that
    /// surfaced as 503). The CC user can retry.
    #[error("Federated Trace mint is temporarily unavailable")]
    UpstreamUnavailable,

    /// Upstream returned 401 — almost always means the shared
    /// `HYPERSAGE_INFRA_KEY` is wrong / out of sync. Mapped to 502
    /// because the CC user's JWT is fine; the bug is on the HS side.
    /// Pages SRE — a single occurrence means rotation drift.
    #[error("Federated Trace upstream rejected our credentials")]
    UpstreamCredentialsRejected,

    #[error("Internal server error")]
    InternalServerError,
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse>
    for LaunchTraceErrors
{
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        // Sub-code stable across error variants for grep-ability in
        // dashboards; per-variant disambiguated by the numeric code.
        let sub_code = "FT"; // Federated Trace
        match self {
            Self::FeatureDisabled => {
                // 404, not 503 — see the docstring for the no-oracle
                // rationale.
                AER::NotFound(ApiError::new(sub_code, 1, self.get_error_message(), None))
            }
            Self::Forbidden => {
                AER::Unauthorized(ApiError::new(sub_code, 2, self.get_error_message(), None))
            }
            Self::UpstreamUnavailable => {
                AER::InternalServerError(ApiError::new(sub_code, 3, self.get_error_message(), None))
            }
            // 502 is mapped via the InternalServerError variant for
            // now — actix_web's ApiErrorResponse doesn't have a
            // dedicated BadGateway today. The structured log line tags
            // these explicitly so SRE can distinguish.
            Self::UpstreamCredentialsRejected => {
                AER::InternalServerError(ApiError::new(sub_code, 4, self.get_error_message(), None))
            }
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, self.get_error_message(), None))
            }
        }
    }
}

impl LaunchTraceErrors {
    pub fn get_error_message(&self) -> String {
        match self {
            // Opaque message: same string for "flag off" and any
            // future "unknown user / unauthorised merchant" surface.
            // Matches HyperSage's collapsed 401 detail policy
            // (see `hypersage/src/web/sessions/federated_resolver.py`
            // `_OPAQUE_401_DETAIL`).
            Self::FeatureDisabled => "Not found".to_string(),
            Self::Forbidden => "Not allowed".to_string(),
            Self::UpstreamUnavailable => {
                "Trace federation temporarily unavailable, please retry".to_string()
            }
            Self::UpstreamCredentialsRejected => "Trace federation misconfigured".to_string(),
            Self::InternalServerError => "Something went wrong".to_string(),
        }
    }
}
