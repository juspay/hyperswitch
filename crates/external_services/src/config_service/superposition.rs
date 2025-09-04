use masking::{ExposeInterface, Secret};

use super::interface::{ConfigContext, ConfigServiceError};

/// Configuration for Superposition integration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SuperpositionConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub token: Secret<String>,
    pub org_id: String,
    pub workspace_id: String,
}

impl Default for SuperpositionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: String::new(),
            token: Secret::new(String::new()),
            org_id: String::new(),
            workspace_id: String::new(),
        }
    }
}

/// Superposition client wrapper
pub struct SuperpositionClient {
    #[cfg(feature = "superposition")]
    client: open_feature::Client,
    #[cfg(not(feature = "superposition"))]
    _phantom: std::marker::PhantomData<()>,
}

impl SuperpositionClient {
    /// Create a new Superposition client
    pub async fn new(
        config: SuperpositionConfig,
    ) -> Result<Self, ConfigServiceError> {
        #[cfg(feature = "superposition")]
        {
            // Initialize OpenFeature client with Superposition provider
            use superposition_provider::{SuperpositionProvider, SuperpositionProviderOptions, RefreshStrategy, PollingStrategy};
            
            let provider_options = SuperpositionProviderOptions {
                endpoint: config.endpoint.clone(),
                token: config.token.expose(),
                org_id: config.org_id.clone(),
                workspace_id: config.workspace_id.clone(),
                fallback_config: None,
                evaluation_cache: None,
                refresh_strategy: RefreshStrategy::Polling(PollingStrategy {
                    interval: 15,
                    timeout: None,
                }),
                experimentation_options: None,
            };
            
            // Create provider and set up OpenFeature
            let provider = SuperpositionProvider::new(provider_options);
            
            router_env::logger::info!("🔄 SUPERPOSITION_CLIENT: Created provider, initializing OpenFeature API...");
            
            // Initialize OpenFeature API and set provider
            let mut api = open_feature::OpenFeature::singleton_mut().await;
            api.set_provider(provider).await;
            
            router_env::logger::info!("🔄 SUPERPOSITION_CLIENT: Provider set, creating client and waiting for initialization...");
            
            // Create client and wait for initialization (as per Superposition examples)
            let client = api.create_client();
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            
            router_env::logger::info!("🎉 SUPERPOSITION_CLIENT: Client created and initialized successfully!");
            
            Ok(Self { client })
        }
        
        #[cfg(not(feature = "superposition"))]
        {
            let _ = config; // Suppress unused variable warning
            Err(ConfigServiceError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }

    /// Get a string value from Superposition
    pub async fn get_string_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> Result<String, ConfigServiceError> {
        #[cfg(feature = "superposition")]
        {
            use open_feature::{EvaluationContext, EvaluationContextFieldValue};
            use std::collections::HashMap;
            
            let evaluation_context = EvaluationContext {
                custom_fields: if let Some(ctx) = context {
                    ctx.values.iter().map(|(k, v)| {
                        (k.clone(), EvaluationContextFieldValue::String(v.clone()))
                    }).collect()
                } else {
                    HashMap::new()
                },
                targeting_key: None,
            };

            self.client
                .get_string_value(key, Some(&evaluation_context), None)
                .await
                .map_err(|e| {
                    ConfigServiceError::SuperpositionError(format!(
                        "Failed to get string value for key '{}': {:?}",
                        key, e
                    ))
                })
        }

        #[cfg(not(feature = "superposition"))]
        {
            let _ = (key, context); // Suppress unused variable warnings
            Err(ConfigServiceError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }

    /// Get a boolean value from Superposition
    pub async fn get_bool_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> Result<bool, ConfigServiceError> {
        #[cfg(feature = "superposition")]
        {
            use open_feature::{EvaluationContext, EvaluationContextFieldValue};
            use std::collections::HashMap;
            
            let evaluation_context = EvaluationContext {
                custom_fields: if let Some(ctx) = context {
                    ctx.values.iter().map(|(k, v)| {
                        (k.clone(), EvaluationContextFieldValue::String(v.clone()))
                    }).collect()
                } else {
                    HashMap::new()
                },
                targeting_key: None,
            };

            router_env::logger::info!(
                "🔍 SUPERPOSITION_CLIENT: Making bool request for key '{}' with context: {:?}, evaluation_context: {:?}",
                key, context, evaluation_context
            );

            let result = self.client
                .get_bool_value(key, Some(&evaluation_context), None)
                .await;

            match &result {
                Ok(value) => {
                    router_env::logger::info!(
                        "✅ SUPERPOSITION_CLIENT: Received bool response for key '{}': {}",
                        key, value
                    );
                }
                Err(e) => {
                    router_env::logger::info!(
                        "❌ SUPERPOSITION_CLIENT: Error response for key '{}': {:?}",
                        key, e
                    );
                }
            }

            result.map_err(|e| {
                ConfigServiceError::SuperpositionError(format!(
                    "Failed to get bool value for key '{}': {:?}",
                    key, e
                ))
            })
        }

        #[cfg(not(feature = "superposition"))]
        {
            let _ = (key, context); // Suppress unused variable warnings
            Err(ConfigServiceError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }

    /// Get an integer value from Superposition
    pub async fn get_int_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> Result<i64, ConfigServiceError> {
        #[cfg(feature = "superposition")]
        {
            use open_feature::{EvaluationContext, EvaluationContextFieldValue};
            use std::collections::HashMap;
            
            let evaluation_context = EvaluationContext {
                custom_fields: if let Some(ctx) = context {
                    ctx.values.iter().map(|(k, v)| {
                        (k.clone(), EvaluationContextFieldValue::String(v.clone()))
                    }).collect()
                } else {
                    HashMap::new()
                },
                targeting_key: None,
            };

            self.client
                .get_int_value(key, Some(&evaluation_context), None)
                .await
                .map_err(|e| {
                    ConfigServiceError::SuperpositionError(format!(
                        "Failed to get int value for key '{}': {:?}",
                        key, e
                    ))
                })
        }

        #[cfg(not(feature = "superposition"))]
        {
            let _ = (key, context); // Suppress unused variable warnings
            Err(ConfigServiceError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }
}

/// Validation for configuration
impl SuperpositionConfig {
    pub fn validate(&self) -> Result<(), ConfigServiceError> {
        if self.enabled {
            if self.endpoint.is_empty() {
                return Err(ConfigServiceError::InvalidConfiguration(
                    "Superposition endpoint cannot be empty".to_string(),
                ));
            }
            if self.org_id.is_empty() {
                return Err(ConfigServiceError::InvalidConfiguration(
                    "Superposition org_id cannot be empty".to_string(),
                ));
            }
            if self.workspace_id.is_empty() {
                return Err(ConfigServiceError::InvalidConfiguration(
                    "Superposition workspace_id cannot be empty".to_string(),
                ));
            }
        }
        Ok(())
    }
}