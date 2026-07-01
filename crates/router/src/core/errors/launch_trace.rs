#[derive(Debug, thiserror::Error)]
pub enum LaunchTraceErrors {
    #[error("Federated Trace sessions are not enabled in this environment")]
    FeatureDisabled,

    #[error("Federated Trace is not available for this role")]
    Forbidden,

    #[error("Trace federation upstream call failed")]
    UpstreamError,

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
            Self::FeatureDisabled => {
                AER::NotFound(ApiError::new(sub_code, 1, self.get_error_message(), None))
            }
            Self::Forbidden => {
                AER::Unauthorized(ApiError::new(sub_code, 2, self.get_error_message(), None))
            }
            Self::UpstreamError | Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, self.get_error_message(), None))
            }
        }
    }
}

impl LaunchTraceErrors {
    pub fn get_error_message(&self) -> String {
        match self {
            Self::FeatureDisabled => "Not found".to_string(),
            Self::Forbidden => "Not allowed".to_string(),
            Self::UpstreamError => {
                "Trace federation temporarily unavailable, please retry".to_string()
            }
            Self::InternalServerError => "Something went wrong".to_string(),
        }
    }
}
