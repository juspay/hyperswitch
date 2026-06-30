#[derive(Debug, thiserror::Error)]
pub enum LaunchTraceErrors {
    #[error("Federated Trace sessions are not enabled in this environment")]
    FeatureDisabled,

    #[error("Federated Trace is not available for this role")]
    Forbidden,

    #[error("Federated Trace mint is temporarily unavailable")]
    UpstreamUnavailable,

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
        let sub_code = "FT";
        match self {
            // 404 (not 503) preserves the no-oracle invariant — flag-off
            // is indistinguishable from "user unknown" upstream.
            Self::FeatureDisabled => {
                AER::NotFound(ApiError::new(sub_code, 1, self.get_error_message(), None))
            }
            Self::Forbidden => {
                AER::Unauthorized(ApiError::new(sub_code, 2, self.get_error_message(), None))
            }
            Self::UpstreamUnavailable => {
                AER::InternalServerError(ApiError::new(sub_code, 3, self.get_error_message(), None))
            }
            // 502-shaped failure mapped to InternalServerError until
            // ApiErrorResponse gains a dedicated BadGateway variant.
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
            // Opaque strings — see no-oracle note on switch() above.
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
