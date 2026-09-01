//! The wire error layer: what a client actually receives.
//!
//! Mirrors `api_models::errors::types` in both shape and file layout. It lives in this crate
//! rather than in `api_models` only because `alerts` has no API-models crate of its own; if one
//! ever appears, this module moves wholesale.
//!
//! Variants here are keyed to **HTTP semantics**, not to causes. Adding a new failure mode
//! usually means a new [`crate::errors::AlertsError`] variant mapping onto an existing one of
//! these, not a new variant here.

use serde::Serialize;

/// The category of an error, as reported to the client.
#[derive(Debug, Serialize)]
pub enum ErrorType {
    /// The request itself was at fault.
    InvalidRequestError,
    /// The service was at fault.
    AlertsError,
}

impl ErrorType {
    /// The string form used in the serialized response body.
    fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidRequestError => "invalid_request",
            Self::AlertsError => "alerts_error",
        }
    }
}

/// The payload carried by every wire error.
///
/// `sub_code` and `error_identifier` combine into a stable, greppable code (`IR_01`) that survives
/// rewording of the message.
#[derive(Debug, Serialize, Clone)]
pub struct ApiError {
    /// Short category prefix, e.g. `IR` for invalid request, `HE` for a service error.
    pub sub_code: &'static str,
    /// Distinguishes errors sharing a `sub_code`.
    pub error_identifier: u16,
    /// Human-readable description. Must never contain a secret or the contents of a request.
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
    /// 500 — the service failed.
    InternalServerError(ApiError),
    /// 401 — authentication failed.
    Unauthorized(ApiError),
}

impl ApiErrorResponse {
    /// The payload of whichever variant this is.
    fn get_internal_error(&self) -> &ApiError {
        match self {
            Self::InternalServerError(error) | Self::Unauthorized(error) => error,
        }
    }

    /// The error category reported to the client.
    fn error_type(&self) -> &'static str {
        match self {
            Self::InternalServerError(_) => ErrorType::AlertsError.as_str(),
            Self::Unauthorized(_) => ErrorType::InvalidRequestError.as_str(),
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
    /// The stable code, e.g. `IR_01`.
    pub code: String,
}

impl From<&ApiErrorResponse> for ErrorResponse {
    fn from(value: &ApiErrorResponse) -> Self {
        let error_info = value.get_internal_error();
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
