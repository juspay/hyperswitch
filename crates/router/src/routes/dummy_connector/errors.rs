#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    ServerNotAvailable,
}

#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = ErrorType)]
pub enum DummyConnectorErrors {
    #[error(error_type = ErrorType::ServerNotAvailable, code = "DC_00", message = "")]
    PaymentStoringError,
}

impl core::fmt::Display for DummyConnectorErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{{"error":{}}}"#,
            serde_json::to_string(self)
                .unwrap_or_else(|_| "Dummy connector error response".to_string())
        )
    }
}

impl common_utils::errors::ErrorSwitch<api_models::errors::types::ApiErrorResponse>
    for DummyConnectorErrors
{
    fn switch(&self) -> api_models::errors::types::ApiErrorResponse {
        use api_models::errors::types::{ApiError, ApiErrorResponse as AER};
        match self {
            Self::PaymentStoringError => AER::InternalServerError(ApiError::new("DC", 0, "", None)),
        }
    }
}
