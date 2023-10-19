use http::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum CustomersErrorResponse {
    #[error("Customer has already been redacted")]
    CustomerRedacted,

    #[error("Something went wrong")]
    InternalServerError,

    #[error("Customer has already been redacted")]
    MandateActive,

    #[error("Customer does not exist in our records")]
    CustomerNotFound,

    #[error("Customer with the given customer id already exists")]
    CustomerAlreadyExists,
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
