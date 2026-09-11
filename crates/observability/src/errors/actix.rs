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
            Self::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        actix_web::HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .body(self.to_string())
    }
}
