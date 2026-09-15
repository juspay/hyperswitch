//! Errors, in three layers.
//!
//! This mirrors the router's layering, which separates errors by *lifetime* and by *audience*:
//!
//! | Layer | Type | Rendered to HTTP? |
//! |---|---|---|
//! | Boot / configuration | [`ConfigurationError`] | never — the process exits instead |
//! | Internal, semantic | [`ObservabilityError`] | no — carried in an [`error_stack::Report`] |
//! | Wire | [`types::ApiErrorResponse`] | yes — via its `ResponseError` impl in [`actix`] |
//!
//! The two request-side layers are bridged by [`common_utils::errors::ErrorSwitch`], which
//! escalates the internal error into the wire error *without consuming the report*. That is the
//! whole point of the split: the full `error_stack` context reaches the log while the client sees
//! only the wire shape, so internal detail cannot leak into a response by accident.

pub mod actix;
pub mod types;

use common_utils::errors::ErrorSwitch;
use diesel_models::errors::DatabaseError;
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
pub type ObservabilityResult<T> = error_stack::Result<T, ConfigurationError>;

/// Errors raised while handling a request.
///
/// Semantic rather than HTTP-shaped: a variant says what went wrong, not what status code the
/// client should see. The mapping happens once, in the [`ErrorSwitch`] impl below, so a handler
/// never has to think about HTTP.
///
/// **A provider refusing a message is not in here.** That is an outcome, reported through
/// [`crate::domain::notifier::Outcome`] and answered with a `200`. What remains is a request we
/// cannot act on, and a notifier that did not work — which is exactly what a `4xx`/`5xx` from this
/// service should mean, so an alert on 5xx pages someone only when the service is genuinely broken.
#[derive(Debug, Error)]
pub enum ObservabilityError {
    /// Something failed that the client can do nothing about.
    #[error("Internal server error")]
    InternalServerError,

    /// The internal API key was missing, malformed, or did not match.
    #[error("Authentication failed")]
    Unauthorized,

    /// The request body was structurally valid but contained unusable values.
    #[error("The request body is invalid")]
    InvalidRequest,

    #[error("The requested resource does not exist")]
    ResourceNotFound,

    #[error("The resource already exists")]
    DuplicateResource,

    /// The path named a destination that is not configured.
    #[error("No destination is configured under `{destination}`")]
    UnknownDestination {
        /// The id the request asked for.
        destination: String,
    },

    /// The provider could not be reached, or answered outside its documented envelope. Nothing is
    /// known about whether the message was delivered, which is what separates this from a refusal.
    #[error("The destination `{destination}` could not be reached")]
    ProviderUnavailable {
        /// The destination that could not be reached.
        destination: String,
    },
}

/// The result type for request handling.
pub type ObservabilityApiResult<T> = error_stack::Result<T, ObservabilityError>;

impl ErrorSwitch<ApiErrorResponse> for ObservabilityError {
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
            Self::InvalidRequest => ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                4,
                "The request body could not be parsed",
            )),
            Self::ResourceNotFound => ApiErrorResponse::NotFound(ApiError::new(
                "HE",
                2,
                "The requested resource does not exist",
            )),
            Self::DuplicateResource => {
                ApiErrorResponse::Conflict(ApiError::new("HE", 1, "The resource already exists"))
            }
            // The id is already in the path the caller sent, so there is nothing to echo back, and
            // the configured ids are deliberately not listed.
            Self::UnknownDestination { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 2, "Unknown destination"))
            }
            // 502 rather than 500: the failure is on the far side of a hop we made. Note this is
            // the *only* provider-shaped error left, because every answer the provider gives is a
            // 200 outcome instead.
            Self::ProviderUnavailable { .. } => ApiErrorResponse::BadGateway(ApiError::new(
                "HE",
                3,
                "The destination could not be reached",
            )),
        }
    }
}

pub trait StorageErrorExt<T, E> {
    #[track_caller]
    fn to_not_found_response(self, not_found_response: E) -> error_stack::Result<T, E>;

    #[track_caller]
    fn to_duplicate_response(self, duplicate_response: E) -> error_stack::Result<T, E>;
}

impl<T> StorageErrorExt<T, ObservabilityError> for diesel_models::StorageResult<T> {
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: ObservabilityError,
    ) -> ObservabilityApiResult<T> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::NotFound => error.change_context(not_found_response),
            _ => error.change_context(ObservabilityError::InternalServerError),
        })
    }

    #[track_caller]
    fn to_duplicate_response(
        self,
        duplicate_response: ObservabilityError,
    ) -> ObservabilityApiResult<T> {
        self.map_err(|error| match error.current_context() {
            DatabaseError::UniqueViolation => error.change_context(duplicate_response),
            _ => error.change_context(ObservabilityError::InternalServerError),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use actix_web::ResponseError;
    use error_stack::report;

    use super::*;

    fn status_of(error: &ObservabilityError) -> u16 {
        ErrorSwitch::<ApiErrorResponse>::switch(error)
            .status_code()
            .as_u16()
    }

    fn code_of(error: &ObservabilityError) -> String {
        let body: serde_json::Value =
            serde_json::from_str(&ErrorSwitch::<ApiErrorResponse>::switch(error).to_string())
                .unwrap();
        body["error"]["code"].as_str().unwrap().to_owned()
    }

    fn storage_failure<T>(context: DatabaseError) -> diesel_models::StorageResult<T> {
        Err(report!(context))
    }

    /// The rule this service is built on: a 5xx means the notifier did not work. Anything the
    /// provider actually said is a 200 and never reaches here.
    #[test]
    fn only_our_own_failures_are_5xx() {
        assert_eq!(
            status_of(&ObservabilityError::ProviderUnavailable {
                destination: "sr_alerts".to_owned(),
            }),
            502
        );
        assert_eq!(status_of(&ObservabilityError::InternalServerError), 500);
    }

    #[test]
    fn a_request_we_cannot_act_on_is_4xx() {
        assert_eq!(
            status_of(&ObservabilityError::UnknownDestination {
                destination: "typo".to_owned(),
            }),
            404
        );
        assert_eq!(status_of(&ObservabilityError::Unauthorized), 401);
        assert_eq!(status_of(&ObservabilityError::InvalidRequest), 400);
        assert_eq!(status_of(&ObservabilityError::ResourceNotFound), 404);
        assert_eq!(status_of(&ObservabilityError::DuplicateResource), 409);
    }

    #[test]
    fn a_missing_row_and_a_duplicate_row_have_stable_codes() {
        assert_eq!(code_of(&ObservabilityError::ResourceNotFound), "HE_02");
        assert_eq!(code_of(&ObservabilityError::DuplicateResource), "HE_01");
    }

    #[test]
    fn a_database_not_found_becomes_the_given_not_found_error() {
        let error = storage_failure::<()>(DatabaseError::NotFound)
            .to_not_found_response(ObservabilityError::ResourceNotFound)
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::ResourceNotFound
        ));
    }

    #[test]
    fn a_unique_violation_becomes_the_given_duplicate_error() {
        let error = storage_failure::<()>(DatabaseError::UniqueViolation)
            .to_duplicate_response(ObservabilityError::DuplicateResource)
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::DuplicateResource
        ));
    }

    #[test]
    fn any_other_database_error_becomes_an_internal_server_error() {
        let not_found = storage_failure::<()>(DatabaseError::DatabaseConnectionError)
            .to_not_found_response(ObservabilityError::ResourceNotFound)
            .unwrap_err();
        let duplicate = storage_failure::<()>(DatabaseError::NotFound)
            .to_duplicate_response(ObservabilityError::DuplicateResource)
            .unwrap_err();

        assert!(matches!(
            not_found.current_context(),
            ObservabilityError::InternalServerError
        ));
        assert!(matches!(
            duplicate.current_context(),
            ObservabilityError::InternalServerError
        ));
    }

    /// A caller that guessed an id should not be handed the registry.
    #[test]
    fn an_unknown_destination_does_not_leak_the_configured_ids() {
        let body =
            ErrorSwitch::<ApiErrorResponse>::switch(&ObservabilityError::UnknownDestination {
                destination: "typo".to_owned(),
            })
            .to_string();

        assert!(body.contains("IR_02"));
        assert!(!body.contains("typo"));
    }
}
