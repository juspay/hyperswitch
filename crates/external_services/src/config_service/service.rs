use std::sync::Arc;

use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};

use super::{
    interface::{ConfigContext, ConfigServiceError, ConfigServiceInterface},
    superposition::{SuperpositionClient, SuperpositionConfig},
};

/// Configuration for the config service
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(tag = "config_service_backend")]
#[serde(rename_all = "snake_case")]
pub enum ConfigServiceConfig {
    Enhanced {
        superposition: SuperpositionConfig,
    },
    #[default]
    Standard,
}


/// Main configuration service implementation  
pub struct ConfigService {
    superposition_client: Option<Arc<SuperpositionClient>>,
}

impl ConfigService {
    /// Create a new configuration service
    pub async fn new(
        config: ConfigServiceConfig,
    ) -> CustomResult<Self, ConfigServiceError> {
        let superposition_client = match config {
            ConfigServiceConfig::Enhanced { superposition } => {
                if superposition.enabled {
                    Some(Arc::new(
                        SuperpositionClient::new(superposition)
                            .await
                            .change_context(ConfigServiceError::SuperpositionError(
                                "Failed to initialize Superposition client".to_string(),
                            ))?,
                    ))
                } else {
                    None
                }
            }
            ConfigServiceConfig::Standard => None,
        };

        Ok(Self {
            superposition_client,
        })
    }

    /// Get configuration with automatic fallback logic (Superposition -> Default)
    async fn get_config_with_fallback<T>(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: T,
        parse_fn: impl Fn(&str) -> Result<T, ConfigServiceError>,
    ) -> CustomResult<T, ConfigServiceError>
    where
        T: Clone + Send + Sync + std::fmt::Debug,
    {
        // 1. Try Superposition first (if enabled)
        if let Some(superposition_client) = &self.superposition_client {
            router_env::logger::info!("🔍 CONFIG_SERVICE: Attempting to get config from Superposition for key '{}'", key);
            match superposition_client
                .get_string_value(key, context.as_ref())
                .await
            {
                Ok(value) => {
                    router_env::logger::info!("✅ CONFIG_SERVICE: Found value in Superposition for key '{}': {:?}", key, value);
                    return parse_fn(&value)
                        .map_err(|e| report!(e))
                        .attach_printable("Failed to convert Superposition value");
                }
                Err(e) => {
                    // Log the error but continue to fallback
                    router_env::logger::info!("❌ CONFIG_SERVICE: Superposition lookup failed for key '{}': {:?}", key, e);
                }
            }
        }

        // 2. Return default value (database integration will be added later)
        router_env::logger::info!("🔧 CONFIG_SERVICE: Using default value for key '{}': {:?}", key, default_value);
        Ok(default_value)
    }
}

#[async_trait::async_trait]
impl ConfigServiceInterface for ConfigService {
    async fn get_config_string(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: String,
    ) -> CustomResult<String, ConfigServiceError> {
        // 1. Try Superposition first (if enabled)
        if let Some(superposition_client) = &self.superposition_client {
            router_env::logger::info!("🔍 CONFIG_SERVICE: Attempting to get string config from Superposition for key '{}'", key);
            match superposition_client
                .get_string_value(key, context.as_ref())
                .await
            {
                Ok(value) => {
                    router_env::logger::info!("✅ CONFIG_SERVICE: Found string value in Superposition for key '{}': '{}'", key, value);
                    return Ok(value);
                }
                Err(e) => {
                    // Log the error but continue to fallback
                    router_env::logger::info!("❌ CONFIG_SERVICE: Superposition string lookup failed for key '{}': {:?}", key, e);
                }
            }
        }

        // 2. Return default value (database integration will be added later)
        router_env::logger::info!("🔧 CONFIG_SERVICE: Using default string value for key '{}': '{}'", key, default_value);
        Ok(default_value)
    }

    async fn get_config_bool(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: bool,
    ) -> CustomResult<bool, ConfigServiceError> {
        // 1. Try Superposition first (if enabled)
        if let Some(superposition_client) = &self.superposition_client {
            router_env::logger::info!("🔍 CONFIG_SERVICE: Attempting to get bool config from Superposition for key '{}'", key);
            match superposition_client
                .get_bool_value(key, context.as_ref())
                .await
            {
                Ok(value) => {
                    router_env::logger::info!("✅ CONFIG_SERVICE: Found bool value in Superposition for key '{}': {}", key, value);
                    return Ok(value);
                }
                Err(e) => {
                    // Log the error but continue to fallback
                    router_env::logger::info!("❌ CONFIG_SERVICE: Superposition bool lookup failed for key '{}': {:?}", key, e);
                }
            }
        }

        // 2. Return default value (database integration will be added later)
        router_env::logger::info!("🔧 CONFIG_SERVICE: Using default bool value for key '{}': {}", key, default_value);
        Ok(default_value)
    }

    async fn get_config_int(
        &self,
        key: &str,
        context: Option<ConfigContext>,
        default_value: i64,
    ) -> CustomResult<i64, ConfigServiceError> {
        // 1. Try Superposition first (if enabled)
        if let Some(superposition_client) = &self.superposition_client {
            router_env::logger::info!("🔍 CONFIG_SERVICE: Attempting to get int config from Superposition for key '{}'", key);
            match superposition_client
                .get_int_value(key, context.as_ref())
                .await
            {
                Ok(value) => {
                    router_env::logger::info!("✅ CONFIG_SERVICE: Found int value in Superposition for key '{}': {}", key, value);
                    return Ok(value);
                }
                Err(e) => {
                    // Log the error but continue to fallback
                    router_env::logger::info!("❌ CONFIG_SERVICE: Superposition int lookup failed for key '{}': {:?}", key, e);
                }
            }
        }

        // 2. Return default value (database integration will be added later)
        router_env::logger::info!("🔧 CONFIG_SERVICE: Using default int value for key '{}': {}", key, default_value);
        Ok(default_value)
    }

}

impl ConfigServiceConfig {
    /// Get the appropriate config service client
    pub async fn get_config_service(
        &self,
    ) -> CustomResult<Arc<ConfigService>, ConfigServiceError> {
        let service = ConfigService::new(self.clone()).await?;
        Ok(Arc::new(service))
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigServiceError> {
        match self {
            ConfigServiceConfig::Enhanced { superposition } => superposition.validate(),
            ConfigServiceConfig::Standard => Ok(()),
        }
    }
}