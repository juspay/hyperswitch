use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum ErrorType {
    InvalidRequestError,
    ObservabilityError,
}

impl ErrorType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidRequestError => "invalid_request",
            Self::ObservabilityError => "observability_error",
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ApiError {
    pub sub_code: &'static str,
    pub error_identifier: u16,
    pub error_message: String,
}

impl ApiError {
    pub fn new(
        sub_code: &'static str,
        error_identifier: u16,
        error_message: impl ToString,
    ) -> Self {
        Self {
            sub_code,
            error_identifier,
            error_message: error_message.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum ApiErrorResponse {
    BadRequest(ApiError),
    Unauthorized(ApiError),
    NotFound(ApiError),
    Conflict(ApiError),
    InternalServerError(ApiError),
    BadGateway(ApiError),
    ServiceUnavailable(ApiError),
}

impl ApiErrorResponse {
    pub(crate) fn payload(&self) -> &ApiError {
        match self {
            Self::BadRequest(error)
            | Self::Unauthorized(error)
            | Self::NotFound(error)
            | Self::Conflict(error)
            | Self::InternalServerError(error)
            | Self::BadGateway(error)
            | Self::ServiceUnavailable(error) => error,
        }
    }

    fn error_type(&self) -> &'static str {
        match self {
            Self::BadRequest(_) | Self::Unauthorized(_) | Self::NotFound(_) | Self::Conflict(_) => {
                ErrorType::InvalidRequestError.as_str()
            }
            Self::InternalServerError(_) | Self::BadGateway(_) | Self::ServiceUnavailable(_) => {
                ErrorType::ObservabilityError.as_str()
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    #[serde(rename = "type")]
    pub error_type: &'static str,
    pub message: String,
    pub code: String,
}

impl From<&ApiErrorResponse> for ErrorResponse {
    fn from(value: &ApiErrorResponse) -> Self {
        let error_info = value.payload();
        Self {
            code: format!("{}_{:02}", error_info.sub_code, error_info.error_identifier),
            message: error_info.error_message.clone(),
            error_type: value.error_type(),
        }
    }
}

impl core::fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let error_response = ErrorResponse::from(self);
        write!(
            f,
            r#"{{"error":{}}}"#,
            serde_json::to_string(&error_response)
                .unwrap_or_else(|_| "API error response".to_string())
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn an_error_serialises_as_a_stable_code_and_a_message() {
        let response = ApiErrorResponse::Unauthorized(ApiError::new("IR", 1, "nope"));
        let body: serde_json::Value = serde_json::from_str(&response.to_string()).unwrap();

        assert_eq!(body["error"]["code"], "IR_01");
        assert_eq!(body["error"]["type"], "invalid_request");
    }

    #[test]
    fn categories_follow_who_the_error_belongs_to() {
        assert_eq!(
            ApiErrorResponse::BadGateway(ApiError::new("HE", 3, "x")).error_type(),
            "observability_error"
        );
        assert_eq!(
            ApiErrorResponse::NotFound(ApiError::new("IR", 2, "x")).error_type(),
            "invalid_request"
        );
    }
}
