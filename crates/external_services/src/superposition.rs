//! Superposition client for dynamic configuration management

use std::collections::HashMap;

use common_utils::errors::CustomResult;
use masking::{ExposeInterface, Secret};
use error_stack::ResultExt;
use open_feature::{EvaluationContext, EvaluationContextFieldValue};
use superposition_provider::{
    EvaluationCacheOptions, PollingStrategy, RefreshStrategy, SuperpositionProvider, 
    SuperpositionProviderOptions,
};

/// Default polling interval in seconds
const fn default_polling_interval() -> u64 {
    15
}

/// Default request timeout (None means no timeout)
const fn default_request_timeout() -> Option<u64> {
    None
}

/// Configuration for Superposition integration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SuperpositionClientConfig {
    /// Whether Superposition is enabled
    pub enabled: bool,
    /// Superposition API endpoint
    pub endpoint: String,
    /// Authentication token for Superposition
    pub token: Secret<String>,
    /// Organization ID in Superposition
    pub org_id: String,
    /// Workspace ID in Superposition
    pub workspace_id: String,
    /// Polling interval in seconds for configuration updates
    #[serde(default = "default_polling_interval")]
    pub polling_interval: u64,
    /// Request timeout in seconds for Superposition API calls (None = no timeout)
    #[serde(default = "default_request_timeout")]
    pub request_timeout: Option<u64>,
}

impl Default for SuperpositionClientConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: String::new(),
            token: Secret::new(String::new()),
            org_id: String::new(),
            workspace_id: String::new(),
            polling_interval: default_polling_interval(),
            request_timeout: default_request_timeout(),
        }
    }
}

/// Errors that can occur when using Superposition
#[derive(Debug, thiserror::Error)]
pub enum SuperpositionError {
    /// Error initializing the Superposition client
    #[error("Failed to initialize Superposition client: {0}")]
    ClientInitError(String),
    /// Error from the Superposition client
    #[error("Superposition client error: {0}")]
    ClientError(String),
    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

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

/// Superposition client wrapper
pub struct SuperpositionClient {
    client: open_feature::Client,
    org_id: String,
    workspace_id: String,
}

impl std::fmt::Debug for SuperpositionClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuperpositionClient")
            .field("org_id", &self.org_id)
            .field("workspace_id", &self.workspace_id)
            .finish_non_exhaustive()
    }
}

impl SuperpositionClient {
    /// Create a new Superposition client
    pub async fn new(config: SuperpositionClientConfig) -> CustomResult<Self, SuperpositionError> {
        if !config.enabled {
            return Err(SuperpositionError::InvalidConfiguration(
                "Superposition is not enabled".to_string(),
            )
            .into());
        }

        let provider_options = SuperpositionProviderOptions {
            endpoint: config.endpoint.clone(),
            token: config.token.expose(),
            org_id: config.org_id.clone(),
            workspace_id: config.workspace_id.clone(),
            fallback_config: None,
            evaluation_cache: None,
            refresh_strategy: RefreshStrategy::Polling(PollingStrategy {
                interval: config.polling_interval,
                timeout: config.request_timeout,
            }),
            experimentation_options: None,
        };

        // Create provider and set up OpenFeature
        let provider = SuperpositionProvider::new(provider_options);

        router_env::logger::info!("Created superposition provider");

        // Initialize OpenFeature API and set provider
        let mut api = open_feature::OpenFeature::singleton_mut().await;
        api.set_provider(provider).await;

        router_env::logger::info!("Set superposition provider, creating client");

        // Create client
        let client = api.create_client();

        router_env::logger::info!("Superposition client initialized successfully");

        Ok(Self {
            client,
            org_id: config.org_id,
            workspace_id: config.workspace_id,
        })
    }

    /// Build evaluation context for Superposition requests
    fn build_evaluation_context(&self, context: Option<&ConfigContext>) -> EvaluationContext {
        EvaluationContext {
            custom_fields: if let Some(ctx) = context {
                ctx.values
                    .iter()
                    .map(|(k, v)| (k.clone(), EvaluationContextFieldValue::String(v.clone())))
                    .collect()
            } else {
                HashMap::new()
            },
            targeting_key: None,
        }
    }

    /// Get a boolean configuration value from Superposition
    pub async fn get_bool_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> CustomResult<bool, SuperpositionError> {
        let evaluation_context = self.build_evaluation_context(context);

        self.client
            .get_bool_value(key, Some(&evaluation_context), None)
            .await
            .map_err(|e| {
                SuperpositionError::ClientError(format!(
                    "Failed to get bool value for key '{}': {:?}",
                    key, e
                ))
            })
            .change_context(SuperpositionError::ClientError(format!(
                "Failed to retrieve bool config for key: {}",
                key
            )))
    }

    /// Get a string configuration value from Superposition
    pub async fn get_string_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> CustomResult<String, SuperpositionError> {
        let evaluation_context = self.build_evaluation_context(context);

        self.client
            .get_string_value(key, Some(&evaluation_context), None)
            .await
            .map_err(|e| {
                SuperpositionError::ClientError(format!(
                    "Failed to get string value for key '{}': {:?}",
                    key, e
                ))
            })
            .change_context(SuperpositionError::ClientError(format!(
                "Failed to retrieve string config for key: {}",
                key
            )))
    }

    /// Get an integer configuration value from Superposition
    pub async fn get_int_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> CustomResult<i64, SuperpositionError> {
        let evaluation_context = self.build_evaluation_context(context);

        self.client
            .get_int_value(key, Some(&evaluation_context), None)
            .await
            .map_err(|e| {
                SuperpositionError::ClientError(format!(
                    "Failed to get int value for key '{}': {:?}",
                    key, e
                ))
            })
            .change_context(SuperpositionError::ClientError(format!(
                "Failed to retrieve int config for key: {}",
                key
            )))
    }
}
