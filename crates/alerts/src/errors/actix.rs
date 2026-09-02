//! The one place the wire error becomes an HTTP response.
//!
//! Kept in its own module, mirroring `api_models::errors::actix`, so that the error *shape*
//! ([`super::types`]) stays independent of the framework rendering it.
//!
//! No `Retry-After` here. A provider rate limiting us is reported as a `200` carrying
//! `retry_after_seconds`, because the notifier reached the provider and did its job; a header that
//! belongs on a `429` has nowhere to sit in that design.

use actix_web::http::header;
use reqwest::StatusCode;

use super::types::ApiErrorResponse;

impl actix_web::ResponseError for ApiErrorResponse {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .body(self.to_string())
    }
}
