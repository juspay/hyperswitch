#[derive(Debug, thiserror::Error)]
pub enum LaunchTraceErrors {
    #[error("Sage is not enabled in this environment")]
    SageDisabled,

    #[error("Sage is not available for this role")]
    Forbidden,

    #[error("Sage call failed")]
    SageError,

    #[error("Internal server error")]
    InternalServerError,
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse>
    for LaunchTraceErrors
{
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        let sub_code = "SG";
        match self {
            Self::SageDisabled => {
                AER::NotFound(ApiError::new(sub_code, 1, self.get_error_message(), None))
            }
            Self::Forbidden => {
                AER::Unauthorized(ApiError::new(sub_code, 2, self.get_error_message(), None))
            }
            Self::SageError | Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("HE", 0, self.get_error_message(), None))
            }
        }
    }
}

impl LaunchTraceErrors {
    pub fn get_error_message(&self) -> String {
        match self {
            Self::SageDisabled => "Not found".to_string(),
            Self::Forbidden => "Not allowed".to_string(),
            Self::SageError => "Sage temporarily unavailable, please retry".to_string(),
            Self::InternalServerError => "Something went wrong".to_string(),
        }
    }
}
