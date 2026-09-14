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

    #[error("The mapper entry is {bytes} bytes, over the {limit} byte limit")]
    EntryTooLarge { bytes: usize, limit: usize },

    #[error("The observability database is unavailable")]
    StorageUnavailable,

    #[error("No mapper entry exists for this name and key")]
    MapperEntryNotFound,

    #[error("No alert definition exists with id `{id}`")]
    DefinitionNotFound { id: String },

    #[error("An alert definition already exists for `{name}` / `{product}`")]
    DuplicateDefinition { name: String, product: String },

    #[error("No alert enablement exists for `{name}` / `{product}`")]
    EnablementNotFound { name: String, product: String },

    #[error("No alert is defined as `{name}` / `{product}`")]
    NotAnAlert { name: String, product: String },

    #[error("The lifecycle write carries {alerts} alerts, over the {limit} allowed")]
    StateTooLarge { alerts: usize, limit: usize },

    #[error("The lifecycle state changed after it was read")]
    StateChanged,

    #[error("No announcement exists with id `{id}`")]
    UnknownAnnouncement { id: String },

    #[error("{alerts} alerts in the lifecycle write belong to another channel")]
    ForeignAlertState { alerts: usize },

    #[error("The announcement window must end after it starts and span at most {max_days} days")]
    InvalidAnnouncementWindow { max_days: i64 },

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

pub trait StorageErrorExt<T, E> {
    #[track_caller]
    fn to_not_found_response(self, not_found_response: E) -> error_stack::Result<T, E>;

    #[track_caller]
    fn to_duplicate_response(self, duplicate_response: E) -> error_stack::Result<T, E>;
}

impl<T> StorageErrorExt<T, ObservabilityError> for error_stack::Result<T, DatabaseError> {
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: ObservabilityError,
    ) -> ObservabilityApiResult<T> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                DatabaseError::NotFound => not_found_response,
                _ => ObservabilityError::InternalServerError,
            };
            err.change_context(new_err)
        })
    }

    #[track_caller]
    fn to_duplicate_response(
        self,
        duplicate_response: ObservabilityError,
    ) -> ObservabilityApiResult<T> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                DatabaseError::UniqueViolation => duplicate_response,
                _ => ObservabilityError::InternalServerError,
            };
            err.change_context(new_err)
        })
    }
}

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
            Self::EntryTooLarge { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "HE",
                3,
                "The mapper entry is larger than this service stores",
            )),
            Self::MapperEntryNotFound => ApiErrorResponse::NotFound(ApiError::new(
                "HE",
                2,
                "Mapper entry does not exist in our records",
            )),
            // The id is already in the path the caller sent, so there is nothing to echo back, and
            // the configured ids are deliberately not listed.
            Self::UnknownDestination { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 2, "Unknown destination"))
            }
            Self::StorageUnavailable => ApiErrorResponse::ServiceUnavailable(ApiError::new(
                "HE",
                0,
                "The observability database is unavailable",
            )),
            Self::DefinitionNotFound { .. } => ApiErrorResponse::NotFound(ApiError::new(
                "HE",
                2,
                "Alert definition does not exist in our records",
            )),
            Self::DuplicateDefinition { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "HE",
                1,
                "The alert definition with the specified name and product already exists in our records",
            )),
            Self::EnablementNotFound { .. } => ApiErrorResponse::NotFound(ApiError::new(
                "HE",
                2,
                "Alert enablement does not exist in our records",
            )),
            Self::NotAnAlert { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "HE",
                3,
                "No alert is defined for this name and product",
            )),
            Self::StateTooLarge { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "HE",
                3,
                "The lifecycle write carries more alerts than this service stores",
            )),
            Self::StateChanged => ApiErrorResponse::Conflict(ApiError::new(
                "IR",
                16,
                "The lifecycle state changed after it was read",
            )),
            Self::UnknownAnnouncement { .. } => ApiErrorResponse::NotFound(ApiError::new(
                "HE",
                2,
                "Announcement does not exist in our records",
            )),
            Self::ForeignAlertState { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "HE",
                3,
                "The lifecycle write names alert state that belongs to another channel",
            )),
            Self::InvalidAnnouncementWindow { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "HE",
                3,
                "The announcement window must end after it starts and span at most 30 days",
            )),
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use actix_web::ResponseError;

    use super::*;

    fn status_of(error: &ObservabilityError) -> u16 {
        ErrorSwitch::<ApiErrorResponse>::switch(error)
            .status_code()
            .as_u16()
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
