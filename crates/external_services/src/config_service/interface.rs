use std::collections::HashMap;

use common_utils::errors::CustomResult;

/// Context for configuration requests
#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
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

/// Errors that can occur in the config service
#[derive(Debug, thiserror::Error)]
pub enum ConfigServiceError {
    #[error("Superposition client error: {0}")]
    SuperpositionError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Interface for configuration service
#[async_trait::async_trait]
pub trait ConfigServiceInterface: Send + Sync {
    /// Get a string configuration value
    async fn get_config_string(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: String,
    ) -> CustomResult<String, ConfigServiceError>;

    /// Get a boolean configuration value
    async fn get_config_bool(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: bool,
    ) -> CustomResult<bool, ConfigServiceError>;

    /// Get an integer configuration value
    async fn get_config_int(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: i64,
    ) -> CustomResult<i64, ConfigServiceError>;

}

