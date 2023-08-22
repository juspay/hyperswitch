use http::StatusCode;

#[derive(Debug, Clone, serde::Serialize)]
pub enum CustomersErrorType {
    ObjectNotFound,
    InvalidRequestError,
    InternalServerError,
}

#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = CustomersErrorType)]
pub enum CustomersErrorResponse {
    #[error(error_type = CustomersErrorType::InvalidRequestError, code = "CE_01", message = "Customer has already been redacted")]
    CustomerRedacted,

    #[error(error_type = CustomersErrorType::InternalServerError, code = "CE_02", message = "Something went wrong")]
    InternalServerError,

    #[error(error_type = CustomersErrorType::InvalidRequestError, code = "CE_03", message = "Customer has already been redacted")]
    MandateActive,

    #[error(error_type = CustomersErrorType::ObjectNotFound, code = "CE_04", message = "Customer does not exist in our records")]
    CustomerNotFound,
}

impl std::fmt::Display for CustomersErrorResponse {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            fmt,
            r#"{{"error":{}}}"#,
            serde_json::to_string(self).unwrap_or_else(|_| "API error response".to_string())
        )
    }
}

impl actix_web::ResponseError for CustomersErrorResponse {
    fn status_code(&self) -> StatusCode {
        common_utils::errors::ErrorSwitch::<api_models::errors::types::ApiErrorResponse>::switch(
            self,
        )
        .status_code()
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        common_utils::errors::ErrorSwitch::<api_models::errors::types::ApiErrorResponse>::switch(
            self,
        )
        .error_response()
    }
}
