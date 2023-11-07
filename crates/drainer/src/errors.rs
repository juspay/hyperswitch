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
    #[error("Unexpected error occurred: {0}")]
    UnexpectedError(String),
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
