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
//! [`Extra`] is the same idea as `api_models`' field of that name: the status code and the stable
//! code say what kind of thing went wrong, and `reason` carries the provider's own word for it so
//! a caller debugging a failed alert does not have to read our logs to find out that Xyne said
//! `channel_not_found`.

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

/// Detail flattened into the error body alongside the code and the message.
///
/// Every field is skipped when absent, so an error that has nothing extra to say serialises
/// exactly as it did before this existed.
#[derive(Debug, Serialize, Clone, Default)]
pub struct Extra {
    /// The destination the request named, echoed back. Present on anything destination-shaped, so
    /// a caller with several configured ids does not have to guess which one failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,

    /// The provider's own error code, verbatim — `channel_not_found`, `msg_too_long`,
    /// `internal_error`. Never a message we invented, so it can be matched on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// How long the provider asked us to wait, when it said. Accompanies a 429.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_seconds: Option<u64>,
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
    /// Machine-readable detail, flattened into the body.
    pub extra: Option<Extra>,
}

impl ApiError {
    /// Construct an [`ApiError`] with no extra detail.
    pub fn new(
        sub_code: &'static str,
        error_identifier: u16,
        error_message: impl ToString,
    ) -> Self {
        Self {
            sub_code,
            error_identifier,
            error_message: error_message.to_string(),
            extra: None,
        }
    }

    /// Attach machine-readable detail.
    pub fn with_extra(mut self, extra: Extra) -> Self {
        self.extra = Some(extra);
        self
    }
}

/// Every error this service can return to a client, keyed by HTTP semantics.
#[derive(Debug, Serialize)]
pub enum ApiErrorResponse {
    /// 400 — the request was wrong in a way its sender can fix.
    ///
    /// Covers a provider refusal that blames the request, not just a malformed body: a message
    /// over the provider's size limit is the caller's problem, and answering 500 would tell them
    /// to retry something that will never succeed.
    BadRequest(ApiError),
    /// 401 — authentication failed.
    Unauthorized(ApiError),
    /// 429 — the provider is rate limiting us.
    TooManyRequests(ApiError),
    /// 500 — the service failed, including where our own configuration is what the provider
    /// rejected. A bad channel id or a revoked token is not something the caller can fix.
    InternalServerError(ApiError),
    /// 502 — the provider could not be reached, or answered something we could not read.
    BadGateway(ApiError),
}

impl ApiErrorResponse {
    /// The payload of whichever variant this is.
    pub(crate) fn get_internal_error(&self) -> &ApiError {
        match self {
            Self::BadRequest(error)
            | Self::Unauthorized(error)
            | Self::TooManyRequests(error)
            | Self::InternalServerError(error)
            | Self::BadGateway(error) => error,
        }
    }

    /// The error category reported to the client.
    fn error_type(&self) -> &'static str {
        match self {
            Self::BadRequest(_) | Self::Unauthorized(_) => ErrorType::InvalidRequestError.as_str(),
            Self::TooManyRequests(_) | Self::InternalServerError(_) | Self::BadGateway(_) => {
                ErrorType::AlertsError.as_str()
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
    /// The stable code, e.g. `IR_01`.
    pub code: String,
    /// Machine-readable detail, flattened so `reason` sits beside `code` rather than nested.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Extra>,
}

impl From<&ApiErrorResponse> for ErrorResponse {
    fn from(value: &ApiErrorResponse) -> Self {
        let error_info = value.get_internal_error();
        Self {
            code: format!("{}_{:02}", error_info.sub_code, error_info.error_identifier),
            message: error_info.error_message.clone(),
            error_type: value.error_type(),
            extra: error_info.extra.clone(),
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
    fn an_error_without_extra_serialises_unchanged() {
        let response = ApiErrorResponse::Unauthorized(ApiError::new("IR", 1, "nope"));
        let body: serde_json::Value = serde_json::from_str(&response.to_string()).unwrap();

        assert_eq!(body["error"]["code"], "IR_01");
        assert_eq!(body["error"]["type"], "invalid_request");
        assert!(body["error"].get("reason").is_none());
        assert!(body["error"].get("destination").is_none());
    }

    #[test]
    fn extra_is_flattened_beside_the_code() {
        let response = ApiErrorResponse::BadRequest(
            ApiError::new("IR", 3, "the provider refused the message").with_extra(Extra {
                destination: Some("sr_alerts".to_owned()),
                reason: Some("msg_too_long".to_owned()),
                ..Default::default()
            }),
        );
        let body: serde_json::Value = serde_json::from_str(&response.to_string()).unwrap();

        assert_eq!(body["error"]["code"], "IR_03");
        assert_eq!(body["error"]["reason"], "msg_too_long");
        assert_eq!(body["error"]["destination"], "sr_alerts");
        assert!(body["error"].get("retry_after_seconds").is_none());
    }
}
