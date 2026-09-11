//! The wire error layer:

use serde::Serialize;

/// The category of an error, as reported to the client.
#[derive(Debug, Serialize)]
pub enum ErrorType {
    /// The request itself was at fault.
    InvalidRequestError,
    /// The service was at fault.
    ObservabilityError,
}

impl ErrorType {
    /// The string form used in the serialized response body.
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidRequestError => "invalid_request",
            Self::ObservabilityError => "observability_error",
        }
    }
}

/// The payload carried by every wire error.
#[derive(Debug, Serialize, Clone)]
pub struct ApiError {
    /// Short category prefix, e.g.
    pub sub_code: &'static str,
    /// Distinguishes errors sharing a `sub_code`.
    pub error_identifier: u16,
    /// Human-readable description.
    pub error_message: String,
}

impl ApiError {
    /// Construct an [`ApiError`].
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

/// Every error this service can return to a client, keyed by HTTP semantics.
#[derive(Debug, Serialize)]
pub enum ApiErrorResponse {
    /// 400 — the request was malformed.
    BadRequest(ApiError),
    /// 401 — authentication failed.
    Unauthorized(ApiError),
    /// 404 — the destination named in the path is not configured.
    NotFound(ApiError),
    /// 500 — the service failed.
    InternalServerError(ApiError),
    /// 502 — the provider could not be reached, or answered outside its documented envelope, so whether the message was delivered is unknown.
    BadGateway(ApiError),
    /// 503 — a dependency this service owns is away, and the condition is expected to clear without intervention.
    ServiceUnavailable(ApiError),
}

impl ApiErrorResponse {
    /// The payload of whichever variant this is.
    pub(crate) fn payload(&self) -> &ApiError {
        match self {
            Self::BadRequest(error)
            | Self::Unauthorized(error)
            | Self::NotFound(error)
            | Self::InternalServerError(error)
            | Self::BadGateway(error)
            | Self::ServiceUnavailable(error) => error,
        }
    }

    /// The error category reported to the client.
    fn error_type(&self) -> &'static str {
        match self {
            Self::BadRequest(_) | Self::Unauthorized(_) | Self::NotFound(_) => {
                ErrorType::InvalidRequestError.as_str()
            }
            Self::InternalServerError(_) | Self::BadGateway(_) | Self::ServiceUnavailable(_) => {
                ErrorType::ObservabilityError.as_str()
            }
        }
    }
}

/// The serialized body, nested under `error` by [`ApiErrorResponse`]'s `Display` impl.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// The error category.
    #[serde(rename = "type")]
    pub error_type: &'static str,
    /// The human-readable description.
    pub message: String,
    /// The stable code, e.g.
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

    /// A notifier that could not reach the provider is our problem; a request we cannot parse is the caller's.
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
