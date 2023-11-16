use std::borrow::Cow;

use reqwest::StatusCode;

#[derive(Debug, serde::Serialize)]
pub enum ErrorType {
    InvalidRequestError,
    RouterError,
    ConnectorError,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct ApiError {
    pub sub_code: &'static str,
    pub error_identifier: u16,
    pub error_message: String,
    pub extra: Option<Extra>,
    #[cfg(feature = "detailed_errors")]
    pub stacktrace: Option<serde_json::Value>,
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
            #[cfg(feature = "detailed_errors")]
            stacktrace: None,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct ErrorResponse<'a> {
    #[serde(rename = "type")]
    error_type: &'static str,
    message: Cow<'a, str>,
    code: String,
    #[serde(flatten)]
    extra: &'a Option<Extra>,
    #[cfg(feature = "detailed_errors")]
    #[serde(skip_serializing_if = "Option::is_none")]
    stacktrace: Option<&'a serde_json::Value>,
}

impl<'a> From<&'a ApiErrorResponse> for ErrorResponse<'a> {
    fn from(value: &'a ApiErrorResponse) -> Self {
        let error_info = value.get_internal_error();
        let error_type = value.error_type();
        Self {
            code: format!("{}_{:02}", error_info.sub_code, error_info.error_identifier),
            message: Cow::Borrowed(value.get_internal_error().error_message.as_str()),
            error_type,
            extra: &error_info.extra,

            #[cfg(feature = "detailed_errors")]
            stacktrace: error_info.stacktrace.as_ref(),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
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

impl serde::ser::Serialize for ApiErrorResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        use serde::ser::SerializeStruct;
        match self {
            Self::Unauthorized(api_error)
            | Self::ForbiddenCommonResource(api_error)
            | Self::ForbiddenPrivateResource(api_error)
            | Self::Conflict(api_error)
            | Self::Gone(api_error)
            | Self::Unprocessable(api_error)
            | Self::InternalServerError(api_error)
            | Self::NotImplemented(api_error)
            | Self::NotFound(api_error)
            | Self::MethodNotAllowed(api_error)
            | Self::BadRequest(api_error)
            | Self::ConnectorError(api_error, _) => {
                let mut state = serializer.serialize_struct("ApiErrorResponse", 2)?;
                state.serialize_field("type", &self.error_type_name())?;
                state.serialize_field("value", api_error)?;
                state.end()

                // serializer.serialize_newtype_variant("ApiErrorResponse", 0, "xyz", i)
            }
        }
    }
}

impl ::core::fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error_response: ErrorResponse<'_> = self.into();
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

    pub fn get_internal_error_mut(&mut self) -> &mut ApiError {
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

    pub(crate) fn error_type(&self) -> &'static str {
        match self {
            Self::Unauthorized(_)
            | Self::ForbiddenCommonResource(_)
            | Self::ForbiddenPrivateResource(_)
            | Self::Conflict(_)
            | Self::Gone(_)
            | Self::Unprocessable(_)
            | Self::NotImplemented(_)
            | Self::MethodNotAllowed(_)
            | Self::NotFound(_)
            | Self::BadRequest(_) => "invalid_request",
            Self::InternalServerError(_) => "api",
            Self::ConnectorError(_, _) => "connector",
        }
    }

    pub(crate) fn error_type_name(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "Unauthorized",
            Self::ForbiddenCommonResource(_) => "ForbiddenCommonResource",
            Self::ForbiddenPrivateResource(_) => "ForbiddenPrivateResource",
            Self::Conflict(_) => "Conflict",
            Self::Gone(_) => "Gone",
            Self::Unprocessable(_) => "Unprocessable",
            Self::NotImplemented(_) => "NotImplemented",
            Self::MethodNotAllowed(_) => "MethodNotAllowed",
            Self::NotFound(_) => "NotFound",
            Self::BadRequest(_) => "BadRequest",
            Self::InternalServerError(_) => "InternalServerError",
            Self::ConnectorError(_, _) => "ConnectorError",
        }
    }
}

impl std::error::Error for ApiErrorResponse {}
