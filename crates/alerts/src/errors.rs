//! Errors, in three layers.
//!
//! This mirrors the router's layering, which separates errors by *lifetime* and by *audience*:
//!
//! | Layer | Type | Rendered to HTTP? |
//! |---|---|---|
//! | Boot / configuration | [`ConfigurationError`] | never — the process exits instead |
//! | Internal, semantic | [`AlertsError`] | no — carried in an [`error_stack::Report`] |
//! | Wire | [`types::ApiErrorResponse`] | yes — via its `ResponseError` impl in [`actix`] |
//!
//! The two request-side layers are bridged by [`common_utils::errors::ErrorSwitch`], which
//! escalates the internal error into the wire error *without consuming the report*. That is the
//! whole point of the split: the full `error_stack` context reaches the log while the client sees
//! only the wire shape, so internal detail cannot leak into a response by accident.

pub mod actix;
pub mod types;

use common_utils::errors::ErrorSwitch;
use thiserror::Error;

use crate::errors::types::{ApiError, ApiErrorResponse, Extra};

/// Errors raised while the application is starting up.
///
/// These are never rendered to a client — by the time a request can arrive, startup has already
/// succeeded. A variant here means the process refuses to start.
#[derive(Debug, Error)]
pub enum ConfigurationError {
    /// A configuration value was present but unusable.
    #[error("Error in parsing config: {0}")]
    ConfigParsingError(String),

    /// The configuration file could not be read or deserialized.
    #[error("Application configuration error: {0}")]
    ConfigurationError(config::ConfigError),

    /// Binding the listener failed, or another I/O error occurred during startup.
    #[error("I/O: {0}")]
    IoError(std::io::Error),
}

impl From<std::io::Error> for ConfigurationError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<config::ConfigError> for ConfigurationError {
    fn from(err: config::ConfigError) -> Self {
        Self::ConfigurationError(err)
    }
}

/// The result type for anything that runs during startup.
pub type AlertsResult<T> = error_stack::Result<T, ConfigurationError>;

/// Errors raised while handling a request.
///
/// Semantic rather than HTTP-shaped: a variant says what went wrong, not what status code the
/// client should see. The mapping to a status code happens once, in the [`ErrorSwitch`] impl
/// below, so a handler never has to think about HTTP.
///
/// The delivery failures are split by **blame**, not by where in the stack they were raised.
/// A provider that refuses a message because it was too long is telling the caller something the
/// caller can act on, so it is a 400; a provider that refuses because our configured channel id is
/// wrong is telling *us* something, so it is a 500 no matter that the same API call produced both.
/// Answering 500 to an oversized message would invite a retry that can never succeed.
#[derive(Debug, Error)]
pub enum AlertsError {
    /// Something failed that the client can do nothing about.
    #[error("Internal server error")]
    InternalServerError,

    /// The internal API key was missing, malformed, or did not match.
    #[error("Authentication failed")]
    Unauthorized,

    /// The request named a destination that is not configured.
    #[error("No destination is configured under `{destination}`")]
    UnknownDestination {
        /// The id the request asked for.
        destination: String,
    },

    /// The provider refused the message for a reason the caller can fix — it was too long, or the
    /// thread it replied to no longer exists.
    #[error("The destination refused the message: {reason}")]
    MessageRejected {
        /// The destination that refused it.
        destination: String,
        /// The provider's own code, kept verbatim so a caller can match on it.
        reason: String,
    },

    /// The provider refused because of how the destination is configured here: an unknown channel,
    /// a credential that is not accepted, a channel the bot was never invited to.
    #[error("The destination `{destination}` is not usable: {reason}")]
    DestinationUnusable {
        /// The destination whose configuration is wrong.
        destination: String,
        /// The provider's own code.
        reason: String,
    },

    /// The provider asked us to slow down.
    #[error("The destination `{destination}` is rate limiting us")]
    RateLimited {
        /// The destination that rate limited us.
        destination: String,
        /// How long it asked us to wait, when it said.
        retry_after_seconds: Option<u64>,
    },

    /// The provider could not be reached, or answered something that is not its documented
    /// envelope. Nothing is known about whether the message was delivered.
    #[error("The destination `{destination}` could not be reached")]
    ProviderUnavailable {
        /// The destination that could not be reached.
        destination: String,
    },
}

/// The result type for request handling.
pub type AlertsApiResult<T> = error_stack::Result<T, AlertsError>;

impl ErrorSwitch<ApiErrorResponse> for AlertsError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::InternalServerError => ApiErrorResponse::InternalServerError(ApiError::new(
                "HE",
                0,
                "Something went wrong",
            )),
            // Deliberately vague. A caller that failed to authenticate learns only that it
            // failed — never whether the header was absent, malformed, or simply wrong.
            Self::Unauthorized => ApiErrorResponse::Unauthorized(ApiError::new(
                "IR",
                1,
                "API key not provided or invalid",
            )),
            // The id is echoed rather than the configured ids listed: a caller that guessed wrong
            // should not be handed the whole registry.
            Self::UnknownDestination { destination } => ApiErrorResponse::BadRequest(
                ApiError::new("IR", 2, "Unknown destination").with_extra(Extra {
                    destination: Some(destination.clone()),
                    ..Default::default()
                }),
            ),
            Self::MessageRejected {
                destination,
                reason,
            } => ApiErrorResponse::BadRequest(
                ApiError::new("IR", 3, "The destination refused the message").with_extra(Extra {
                    destination: Some(destination.clone()),
                    reason: Some(reason.clone()),
                    ..Default::default()
                }),
            ),
            Self::RateLimited {
                destination,
                retry_after_seconds,
            } => ApiErrorResponse::TooManyRequests(
                ApiError::new("HE", 4, "The destination is rate limiting us").with_extra(Extra {
                    destination: Some(destination.clone()),
                    retry_after_seconds: *retry_after_seconds,
                    ..Default::default()
                }),
            ),
            Self::DestinationUnusable {
                destination,
                reason,
            } => ApiErrorResponse::InternalServerError(
                ApiError::new("HE", 2, "The destination is not usable").with_extra(Extra {
                    destination: Some(destination.clone()),
                    reason: Some(reason.clone()),
                    ..Default::default()
                }),
            ),
            Self::ProviderUnavailable { destination } => ApiErrorResponse::BadGateway(
                ApiError::new("HE", 3, "The destination could not be reached").with_extra(Extra {
                    destination: Some(destination.clone()),
                    ..Default::default()
                }),
            ),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use actix_web::ResponseError;

    use super::*;

    fn status_of(error: &AlertsError) -> u16 {
        ErrorSwitch::<ApiErrorResponse>::switch(error)
            .status_code()
            .as_u16()
    }

    /// The blame split, asserted directly: the same provider call produces both of these and they
    /// must not land on the same status.
    #[test]
    fn refusal_blames_the_caller_and_misconfiguration_blames_us() {
        assert_eq!(
            status_of(&AlertsError::MessageRejected {
                destination: "sr_alerts".to_owned(),
                reason: "msg_too_long".to_owned(),
            }),
            400
        );
        assert_eq!(
            status_of(&AlertsError::DestinationUnusable {
                destination: "sr_alerts".to_owned(),
                reason: "channel_not_found".to_owned(),
            }),
            500
        );
    }

    #[test]
    fn transport_and_rate_limiting_have_their_own_statuses() {
        assert_eq!(
            status_of(&AlertsError::ProviderUnavailable {
                destination: "sr_alerts".to_owned(),
            }),
            502
        );
        assert_eq!(
            status_of(&AlertsError::RateLimited {
                destination: "sr_alerts".to_owned(),
                retry_after_seconds: Some(30),
            }),
            429
        );
    }

    #[test]
    fn a_rate_limit_carries_retry_after_as_a_header() {
        let response = ErrorSwitch::<ApiErrorResponse>::switch(&AlertsError::RateLimited {
            destination: "sr_alerts".to_owned(),
            retry_after_seconds: Some(30),
        })
        .error_response();

        assert_eq!(
            response
                .headers()
                .get(actix_web::http::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok()),
            Some("30")
        );
    }

    #[test]
    fn an_unknown_destination_does_not_leak_the_registry() {
        let body = ErrorSwitch::<ApiErrorResponse>::switch(&AlertsError::UnknownDestination {
            destination: "typo".to_owned(),
        })
        .to_string();

        assert!(body.contains("IR_02"));
        assert!(body.contains("typo"));
    }
}
