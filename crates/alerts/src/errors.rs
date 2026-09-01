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

use crate::errors::types::{ApiError, ApiErrorResponse};

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
#[derive(Debug, Error)]
pub enum AlertsError {
    /// The internal API key was missing, malformed, or did not match.
    #[error("Authentication failed")]
    Unauthorized,

    /// Something failed that the client can do nothing about.
    #[error("Internal server error")]
    InternalServerError,
}

/// The result type for request handling.
pub type AlertsApiResult<T> = error_stack::Result<T, AlertsError>;

impl ErrorSwitch<ApiErrorResponse> for AlertsError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            // Deliberately vague. A caller that failed to authenticate learns only that it
            // failed — never whether the header was absent, malformed, or simply wrong.
            Self::Unauthorized => ApiErrorResponse::Unauthorized(ApiError::new(
                "IR",
                1,
                "API key not provided or invalid",
            )),
            Self::InternalServerError => ApiErrorResponse::InternalServerError(ApiError::new(
                "HE",
                0,
                "Something went wrong",
            )),
        }
    }
}
