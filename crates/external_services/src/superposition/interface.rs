use std::collections::HashMap;

use common_utils::errors::CustomResult;

/// Context for configuration requests
#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
    /// Key-value pairs for configuration context
    pub values: HashMap<String, String>,
}

impl ConfigContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a key-value pair to the context
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }
}

/// Errors that can occur in the superposition service
#[derive(Debug, thiserror::Error)]
pub enum SuperpositionError {
    /// Error from the Superposition client
    #[error("Superposition client error: {0}")]
    ClientError(String),
    /// Error during serialization/deserialization
    #[error("Serialization error: {0}")]
    SerializationError(String),
    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Interface for superposition service
#[async_trait::async_trait]
pub trait SuperpositionInterface: Send + Sync {
    /// Get a string configuration value
    async fn get_config_string(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: String,
    ) -> CustomResult<String, SuperpositionError>;

    /// Get a boolean configuration value
    async fn get_config_bool(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: bool,
    ) -> CustomResult<bool, SuperpositionError>;

    /// Get an integer configuration value
    async fn get_config_int(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: i64,
    ) -> CustomResult<i64, SuperpositionError>;
}
