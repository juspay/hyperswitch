use redis_interface as redis;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DrainerError {
    #[error("Error in parsing config : {0}")]
    ConfigParsingError(String),
    #[error("Error during redis operation : {0:?}")]
    RedisError(error_stack::Report<redis::errors::RedisError>),
    #[error("Application configuration error: {0}")]
    ConfigurationError(config::ConfigError),
    #[error("Error while configuring signals: {0}")]
    SignalError(String),
    #[error("Error while parsing data from the stream: {0:?}")]
    ParsingError(error_stack::Report<common_utils::errors::ParsingError>),
    #[error("Unexpected error occurred: {0}")]
    UnexpectedError(String),
    #[error("I/O: {0}")]
    IoError(std::io::Error),
}

#[derive(Debug, Error, Clone, serde::Serialize)]
pub enum HealthCheckError {
    #[error("Database health check is failiing with error: {message}")]
    DbError { message: String },
    #[error("Redis health check is failiing with error: {message}")]
    RedisError { message: String },
}

impl From<std::io::Error> for DrainerError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub type DrainerResult<T> = error_stack::Result<T, DrainerError>;

impl From<config::ConfigError> for DrainerError {
    fn from(err: config::ConfigError) -> Self {
        Self::ConfigurationError(err)
    }
}

impl From<error_stack::Report<redis::errors::RedisError>> for DrainerError {
    fn from(err: error_stack::Report<redis::errors::RedisError>) -> Self {
        Self::RedisError(err)
    }
}

impl actix_web::ResponseError for HealthCheckError {
    fn status_code(&self) -> reqwest::StatusCode {
        use reqwest::StatusCode;

        match self {
            Self::DbError { .. } | Self::RedisError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
