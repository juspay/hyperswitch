use reqwest::StatusCode;

#[derive(Debug, serde::Serialize)]
pub enum ErrorType {
    InvalidRequestError,
    HyperswitchError,
    ConnectorError,
}

#[derive(Debug, serde::Serialize)]
pub struct ApiError {
    pub sub_code: &'static str,
    pub error_identifier: u16,
    pub error_message: String,
    pub extra: Option<Extra>,
}

impl ApiError {
    pub fn new(
        sub_code: &'static str,
        error_identifier: u16,
        error_message: impl ToString,
        extra: Option<Extra>,
    ) -> Self {
        Self {
            sub_code,
            error_identifier,
            error_message: error_message.to_string(),
            extra,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct ErrorResponse {
    error_type: String,
    error_message: String,
    error_code: String,
    #[serde(flatten)]
    extra: Extra,
}

impl From<&ApiErrorResponse> for ErrorResponse {
    fn from(value: &ApiErrorResponse) -> Self {
        let error_info = value.get_internal_error();
        let error_type = value.error_type().to_string();
        Self {
            error_code: format!("{}_{}", error_info.sub_code, error_info.error_identifier),
            error_message: error_info.error_message.clone(),
            error_type,
            extra: error_info.extra.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, serde::Serialize, Default, Clone)]
pub struct Extra {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector: Option<String>,
}

#[derive(Debug)]
pub enum ApiErrorResponse {
    Unauthorized(ApiError),
    ForbiddenCommonResource(ApiError),
    ForbiddenPrivateResource(ApiError),
    Conflict(ApiError),
    Gone(ApiError),
    Unprocessable(ApiError),
    InternalServerError(ApiError),
    NotImplemented(ApiError),
    ConnectorError(ApiError, StatusCode),
    NotFound(ApiError),
    MethodNotAllowed(ApiError),
    BadRequest(ApiError),
}

impl ::core::fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_response: ErrorResponse = self.into();
        write!(
            f,
            r#"{{"error":{}}}"#,
            serde_json::to_string(&error_response)
                .unwrap_or_else(|_| "API error response".to_string())
        )
    }
}

impl ApiErrorResponse {
    pub(crate) fn get_internal_error(&self) -> &ApiError {
        match self {
            Self::Unauthorized(i)
            | Self::ForbiddenCommonResource(i)
            | Self::ForbiddenPrivateResource(i)
            | Self::Conflict(i)
            | Self::Gone(i)
            | Self::Unprocessable(i)
            | Self::InternalServerError(i)
            | Self::NotImplemented(i)
            | Self::NotFound(i)
            | Self::MethodNotAllowed(i)
            | Self::BadRequest(i)
            | Self::ConnectorError(i, _) => i,
        }
    }

    pub(crate) fn error_type(&self) -> &str {
        match self {
            Self::Unauthorized(_)
            | Self::ForbiddenCommonResource(_)
            | Self::ForbiddenPrivateResource(_)
            | Self::Conflict(_)
            | Self::Gone(_)
            | Self::Unprocessable(_)
            | Self::NotImplemented(_) => "invalid_request",
            Self::InternalServerError(_) => "api",
            Self::ConnectorError(_, _) => "connector",
            Self::MethodNotAllowed(_) => "invalid_request",
            Self::NotFound(_) => "invalid_request",
            Self::BadRequest(_) => "invalid_request",
        }
    }
}

impl std::error::Error for ApiErrorResponse {}
