use http::StatusCode;


#[derive(Debug, Clone, serde::Serialize)]
pub enum CustomersErrorType {
    ObjectNotFound,
    InternalServerError
}

#[derive(Debug, Clone, router_derive::ApiError)]
#[error(error_type_enum = CustomersErrorType)]
pub enum CustomersErrorResponse {
    #[error(error_type = CustomersErrorType::ObjectNotFound, code = "CE_01", message = "Customer not found")]
    CustomerNotFound,
    
    #[error(error_type = CustomersErrorType::InternalServerError, code = "CE_02", message = "Customer has already been redacted")]
    CustomerRedacted,
    
    #[error(error_type = CustomersErrorType::InternalServerError, code = "CE_03", message = "Something went wrong")]
    InternalServerError,
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
