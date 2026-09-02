//! The wire error layer: what a client actually receives.
//!
//! Mirrors `api_models::errors::types` in both shape and file layout. It lives in this crate
//! rather than in `api_models` only because `alerts` has no API-models crate of its own; if one
//! ever appears, this module moves wholesale.
//!
//! Variants here are keyed to **HTTP semantics**, not to causes. Adding a new failure mode
//! usually means a new [`crate::errors::AlertsError`] variant mapping onto an existing one of
//! these, not a new variant here.
//!
//! **A provider refusing a message does not reach this module.** The status code answers "did the
//! notifier function?", and a refusal means it did. Refusals are reported as a normal `200`
//! response — see [`crate::types`] — which is why nothing here carries a provider's error code.

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
///
/// Deliberately short. Everything a provider says about a message is a `200`, so the only failures
/// left are a request we cannot act on, and a notifier that did not work.
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
    /// 502 — the provider could not be reached, or answered outside its documented envelope, so
    /// whether the message was delivered is unknown.
    BadGateway(ApiError),
}

impl ApiErrorResponse {
    /// The payload of whichever variant this is.
    ///
    /// Named for the payload rather than for "internal", which `api_models` uses and which reads
    /// as *internal server error* at a glance.
    pub(crate) fn payload(&self) -> &ApiError {
        match self {
            Self::BadRequest(error)
            | Self::Unauthorized(error)
            | Self::NotFound(error)
            | Self::InternalServerError(error)
            | Self::BadGateway(error) => error,
        }
    }

    /// The error category reported to the client.
    ///
    /// Mirrors `api_models::errors::types::ApiErrorResponse::error_type`.
    fn error_type(&self) -> &'static str {
        match self {
            Self::BadRequest(_) | Self::Unauthorized(_) | Self::NotFound(_) => {
                ErrorType::InvalidRequestError.as_str()
            }
            Self::InternalServerError(_) | Self::BadGateway(_) => ErrorType::AlertsError.as_str(),
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

    /// A notifier that could not reach the provider is our problem; a request we cannot parse is
    /// the caller's. Nothing in between reaches this module.
    #[test]
    fn categories_follow_who_the_error_belongs_to() {
        assert_eq!(
            ApiErrorResponse::BadGateway(ApiError::new("HE", 3, "x")).error_type(),
            "alerts_error"
        );
        assert_eq!(
            ApiErrorResponse::NotFound(ApiError::new("IR", 2, "x")).error_type(),
            "invalid_request"
        );
    }
}
