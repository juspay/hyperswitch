//! The one place the wire error becomes an HTTP response.
//!
//! Kept in its own module, mirroring `api_models::errors::actix`, so that the error *shape*
//! ([`super::types`]) stays independent of the framework rendering it.

use super::types::ApiErrorResponse;

impl actix_web::ResponseError for ApiErrorResponse {
    fn status_code(&self) -> reqwest::StatusCode {
        use reqwest::StatusCode;

        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::http::header;

        actix_web::HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .body(self.to_string())
    }
}
