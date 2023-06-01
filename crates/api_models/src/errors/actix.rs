use super::types::ApiErrorResponse;

impl actix_web::ResponseError for ApiErrorResponse {
    fn status_code(&self) -> reqwest::StatusCode {
        use reqwest::StatusCode;

        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED, // 401
            Self::ForbiddenCommonResource(_) => StatusCode::FORBIDDEN, // 403
            Self::ForbiddenPrivateResource(_) => StatusCode::NOT_FOUND, // 404
            Self::Conflict(_) => StatusCode::CONFLICT,         // 409
            Self::Gone(_) => StatusCode::GONE,                 // 410
            Self::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY, // 422
            Self::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR, // 500
            Self::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED, // 501
            Self::ConnectorError(_, code) => *code, // whatever the connector throws , if not there 500
            Self::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED, // 405
            Self::NotFound(_) => StatusCode::NOT_FOUND, // 404
            Self::BadRequest(_) => StatusCode::BAD_REQUEST, // 400
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::http::header;

        actix_web::HttpResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, mime::APPLICATION_JSON))
            .body(self.to_string())
    }
}
