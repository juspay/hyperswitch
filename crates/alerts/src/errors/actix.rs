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
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        let mut builder = actix_web::HttpResponseBuilder::new(self.status_code());
        builder.insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON));

        // A 429 carries the wait as a header as well as in the body. The body field is what R
        // reads, since it parses JSON anyway; the header is what every generic HTTP client and
        // proxy between here and there already understands.
        if let Some(seconds) = self
            .get_internal_error()
            .extra
            .as_ref()
            .and_then(|extra| extra.retry_after_seconds)
        {
            builder.insert_header((header::RETRY_AFTER, seconds));
        }

        builder.body(self.to_string())
    }
}
