//! Errors, in three layers, mirroring the router's split by lifetime and audience.
//!
//! | Layer | Type | Rendered to HTTP? |
//! |---|---|---|
//! | Boot / configuration | [`ConfigurationError`] | never - the process exits instead |
//! | Internal, semantic | [`ObservabilityError`] | no - carried in an [`error_stack::Report`] |
//! | Wire | [`types::ApiErrorResponse`] | yes - via its `ResponseError` impl in [`actix`] |
//!
//! [`common_utils::errors::ErrorSwitch`] bridges the two request-side layers without consuming the
//! report, so the full `error_stack` context reaches the log while the client sees only the wire
//! shape - internal detail cannot leak into a response by accident.

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

    /// The observability database could not be reached, or a query against it failed.
    ///
    /// Distinct from an empty result on purpose. The alert manager's own outage rule reads "no
    /// alerts" as "nothing is wrong", so a list route that answered `200 []` when its database was
    /// down would report all-clear during exactly the incident it exists to catch.
    #[error("The observability database is unavailable")]
    StorageUnavailable,

    /// No alert definition exists with the requested id.
    #[error("No alert definition exists with id `{id}`")]
    DefinitionNotFound {
        /// The id the request asked for.
        id: String,
    },

    /// A definition already exists for this name and product.
    #[error("An alert definition already exists for `{name}` / `{product}`")]
    DuplicateDefinition {
        /// The name that collided.
        name: String,
        /// The product it collided within.
        product: String,
    },

    /// No enablement row exists for this name and product.
    #[error("No alert enablement exists for `{name}` / `{product}`")]
    EnablementNotFound {
        /// The name the request asked for.
        name: String,
        /// The product the request asked for.
        product: String,
    },

    /// The name and product do not identify an alert that can be switched on or off.
    ///
    /// Either no definition exists for the pair, or it names the reserved `all` definition, which
    /// carries suppression for every detector and is not itself a detector. r-apps enforces the
    /// first half with a database trigger we do not have; the check lives here instead, so that an
    /// enablement row cannot name an alert that does not exist.
    #[error("No alert is defined as `{name}` / `{product}`")]
    NotAnAlert {
        /// The name the request asked for.
        name: String,
        /// The product the request asked for.
        product: String,
    },

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
            // The id is already in the path the caller sent, so there is nothing to echo back, and
            // the configured ids are deliberately not listed.
            Self::UnknownDestination { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 2, "Unknown destination"))
            }
            // 503 rather than 500: this service is fine, and the condition is expected to clear
            // without anyone touching it — the same reading `/health/ready` takes. The failing
            // host, database and role stay in the log.
            Self::StorageUnavailable => ApiErrorResponse::ServiceUnavailable(ApiError::new(
                "HE",
                1,
                "The observability database is unavailable",
            )),
            // The id is already in the path the caller sent, so there is nothing to echo back.
            Self::DefinitionNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 3, "Unknown alert definition"))
            }
            // A name identifies a definition to the alert manager and to the enablement table, so
            // a second one under the same name is refused rather than silently shadowing the
            // first.
            Self::DuplicateDefinition { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                5,
                "An alert definition already exists for this name and product",
            )),
            Self::EnablementNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 6, "Unknown alert enablement"))
            }
            // 400 rather than 404: the missing thing is the definition the body named, not the
            // enablement row the path addresses, and answering 404 would read as "this switch does
            // not exist" when the problem is that the alert does not.
            Self::NotAnAlert { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                7,
                "No alert is defined for this name and product",
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

    /// A database that is away is not this service being broken, and must not be alerted on as if
    /// it were. It is also not an empty list — the distinction the alert manager's outage rule
    /// depends on.
    #[test]
    fn an_unreachable_database_is_503_and_not_500() {
        assert_eq!(status_of(&ObservabilityError::StorageUnavailable), 503);
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
        assert_eq!(
            status_of(&ObservabilityError::DefinitionNotFound {
                id: "0189d0a0-0000-7000-8000-000000000000".to_owned(),
            }),
            404
        );
        assert_eq!(
            status_of(&ObservabilityError::EnablementNotFound {
                name: "sr_drop".to_owned(),
                product: "payments".to_owned(),
            }),
            404
        );
        assert_eq!(
            status_of(&ObservabilityError::DuplicateDefinition {
                name: "sr_drop".to_owned(),
                product: "payments".to_owned(),
            }),
            400
        );
    }

    /// Every condition a caller can provoke has to be told apart from every other one by the code
    /// alone, because the messages are free to be reworded and the codes are not.
    #[test]
    fn no_two_conditions_share_a_code() {
        let codes = [
            ObservabilityError::InternalServerError,
            ObservabilityError::Unauthorized,
            ObservabilityError::InvalidRequest,
            ObservabilityError::StorageUnavailable,
            ObservabilityError::DefinitionNotFound { id: String::new() },
            ObservabilityError::DuplicateDefinition {
                name: String::new(),
                product: String::new(),
            },
            ObservabilityError::EnablementNotFound {
                name: String::new(),
                product: String::new(),
            },
            ObservabilityError::NotAnAlert {
                name: String::new(),
                product: String::new(),
            },
            ObservabilityError::UnknownDestination {
                destination: String::new(),
            },
            ObservabilityError::ProviderUnavailable {
                destination: String::new(),
            },
        ]
        .iter()
        .map(|error| {
            let payload = ErrorSwitch::<ApiErrorResponse>::switch(error);
            format!(
                "{}_{:02}",
                payload.payload().sub_code,
                payload.payload().error_identifier
            )
        })
        .collect::<std::collections::HashSet<_>>();

        assert_eq!(codes.len(), 10);
    }

    /// The names a caller guessed are theirs already; the ones that exist are not.
    #[test]
    fn a_missing_definition_does_not_echo_the_key_back() {
        let body = ErrorSwitch::<ApiErrorResponse>::switch(&ObservabilityError::NotAnAlert {
            name: "typo".to_owned(),
            product: "payments".to_owned(),
        })
        .to_string();

        assert!(body.contains("IR_07"));
        assert!(!body.contains("typo"));
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
