pub mod actix;
pub mod types;

use common_utils::errors::ErrorSwitch;
use thiserror::Error;

use crate::errors::types::{ApiError, ApiErrorResponse};

#[derive(Debug, Error)]
pub enum ConfigurationError {
    #[error("Error in parsing config: {0}")]
    ConfigParsingError(String),

    #[error("Application configuration error: {0}")]
    ConfigurationError(config::ConfigError),

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

pub type ObservabilityResult<T> = error_stack::Result<T, ConfigurationError>;

#[derive(Debug, Error)]
pub enum ObservabilityError {
    #[error("Internal server error")]
    InternalServerError,

    #[error("Authentication failed")]
    Unauthorized,

    #[error("The request body is invalid")]
    InvalidRequest,

    #[error("The observability database is unavailable")]
    StorageUnavailable,

    #[error("No alert definition exists with id `{id}`")]
    DefinitionNotFound { id: String },

    #[error("An alert definition already exists for `{name}` / `{product}`")]
    DuplicateDefinition { name: String, product: String },

    #[error("No alert enablement exists for `{name}` / `{product}`")]
    EnablementNotFound { name: String, product: String },

    #[error("No alert is defined as `{name}` / `{product}`")]
    NotAnAlert { name: String, product: String },

    #[error("No destination is configured under `{destination}`")]
    UnknownDestination { destination: String },

    #[error("The destination `{destination}` could not be reached")]
    ProviderUnavailable { destination: String },
}

pub type ObservabilityApiResult<T> = error_stack::Result<T, ObservabilityError>;

impl ErrorSwitch<ApiErrorResponse> for ObservabilityError {
    fn switch(&self) -> ApiErrorResponse {
        match self {
            Self::InternalServerError => ApiErrorResponse::InternalServerError(ApiError::new(
                "HE",
                0,
                "Something went wrong",
            )),
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
            Self::UnknownDestination { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 2, "Unknown destination"))
            }
            Self::StorageUnavailable => ApiErrorResponse::ServiceUnavailable(ApiError::new(
                "HE",
                1,
                "The observability database is unavailable",
            )),
            Self::DefinitionNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 3, "Unknown alert definition"))
            }
            Self::DuplicateDefinition { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                5,
                "An alert definition already exists for this name and product",
            )),
            Self::EnablementNotFound { .. } => {
                ApiErrorResponse::NotFound(ApiError::new("IR", 6, "Unknown alert enablement"))
            }
            Self::NotAnAlert { .. } => ApiErrorResponse::BadRequest(ApiError::new(
                "IR",
                7,
                "No alert is defined for this name and product",
            )),
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
