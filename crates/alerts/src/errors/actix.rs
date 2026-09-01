//! The one place the wire error becomes an HTTP response.
//!
//! Kept in its own module, mirroring `api_models::errors::actix`, so that the error *shape*
//! ([`super::types`]) stays independent of the framework rendering it.

use actix_web::http::header;
use reqwest::StatusCode;

use super::types::ApiErrorResponse;

impl actix_web::ResponseError for ApiErrorResponse {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .body(self.to_string())
    }
}
