#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ErrorType {
    InvalidRequestError,
    ObjectNotFound,
    ProcessingError,
    ServerNotAvailable,
    ValidationError,
}

#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = ErrorType)]
#[allow(dead_code)]
pub enum DummyConnectorErrors {
    #[error(error_type = ErrorType::ServerNotAvailable, code = "DC_00", message = "Something went wrong")]
    InternalServerError,

    #[error(error_type = ErrorType::InvalidRequestError, code = "DC_01", message = "Missing required param: {field_name}")]
    MissingRequiredField { field_name: &'static str },

    #[error(error_type = ErrorType::ObjectNotFound, code = "DC_02", message = "Payment does not exist in our records")]
    PaymentNotFound,

    #[error(error_type = ErrorType::InvalidRequestError, code = "DC_03", message = "Refund amount exceeds the payment amount")]
    RefundAmountExceedsPaymentAmount,

    #[error(error_type = ErrorType::ServerNotAvailable, code = "DC_04", message = "")]
    PaymentStoringError,
}

impl ::core::fmt::Display for DummyConnectorErrors {
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
            Self::InternalServerError => {
                AER::InternalServerError(ApiError::new("DC", 0, "Something went wrong", None))
            }
            Self::MissingRequiredField { field_name } => AER::InternalServerError(ApiError::new(
                "DC",
                1,
                format!("Missing required param: {field_name}"),
                None,
            )),
            Self::PaymentNotFound => AER::InternalServerError(ApiError::new(
                "DC",
                0,
                "Payment does not exist in our records",
                None,
            )),
            Self::RefundAmountExceedsPaymentAmount => AER::InternalServerError(ApiError::new(
                "DC",
                3,
                "Refund amount exceeds the payment amount",
                None,
            )),
            Self::PaymentStoringError => AER::InternalServerError(ApiError::new("DC", 4, "", None)),
        }
    }
}
