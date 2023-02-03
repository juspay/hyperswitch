pub enum ErrorType {
    InvalidRequestError,
    RouterError,
    ConnectorError,
}

#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub sub_code: &'static str,
    pub error_identifier: u8,
    pub error_message: &'static str,
}

#[derive(Debug, serde::Serialize)]
pub enum ApiErrorResponse {
    Unauthorized(ApiError),
    ForbiddenCommonResource(ApiError),
    ForbiddenPrivateResource(ApiError),
    Conflict(ApiError),
    Gone(ApiError),
    Unprocessable(ApiError),
    InternalServerError(ApiError),
    NotImplemented(ApiError),
}

impl ::core::fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{{"error":{}}}"#,
            serde_json::to_string(self).unwrap_or_else(|_| "API error response".to_string())
        )
    }
}
