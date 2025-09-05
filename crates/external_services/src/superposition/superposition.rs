use masking::{ExposeInterface, Secret};

use super::interface::{ConfigContext, SuperpositionError};

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
}

impl Default for SuperpositionClientConfig {
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

impl std::fmt::Debug for SuperpositionClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SuperpositionClient")
            .finish_non_exhaustive()
    }
}

impl SuperpositionClient {
    /// Create a new Superposition client
    pub async fn new(
        config: SuperpositionClientConfig,
    ) -> Result<Self, SuperpositionError> {
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
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }

    /// Get a string value from Superposition
    pub async fn get_string_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> Result<String, SuperpositionError> {
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
                    SuperpositionError::ClientError(format!(
                        "Failed to get string value for key '{}': {:?}",
                        key, e
                    ))
                })
        }

        #[cfg(not(feature = "superposition"))]
        {
            let _ = (key, context); // Suppress unused variable warnings
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }

    /// Get a boolean value from Superposition
    pub async fn get_bool_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> Result<bool, SuperpositionError> {
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
                SuperpositionError::ClientError(format!(
                    "Failed to get bool value for key '{}': {:?}",
                    key, e
                ))
            })
        }

        #[cfg(not(feature = "superposition"))]
        {
            let _ = (key, context); // Suppress unused variable warnings
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }

    /// Get an integer value from Superposition
    pub async fn get_int_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> Result<i64, SuperpositionError> {
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
                    SuperpositionError::ClientError(format!(
                        "Failed to get int value for key '{}': {:?}",
                        key, e
                    ))
                })
        }

        #[cfg(not(feature = "superposition"))]
        {
            let _ = (key, context); // Suppress unused variable warnings
            Err(SuperpositionError::InvalidConfiguration(
                "Superposition feature is not enabled".to_string(),
            ))
        }
    }
}

/// Validation for configuration
impl SuperpositionClientConfig {
    /// Validate the Superposition configuration
    pub fn validate(&self) -> Result<(), SuperpositionError> {
        if self.enabled {
            if self.endpoint.is_empty() {
                return Err(SuperpositionError::InvalidConfiguration(
                    "Superposition endpoint cannot be empty".to_string(),
                ));
            }
            if self.org_id.is_empty() {
                return Err(SuperpositionError::InvalidConfiguration(
                    "Superposition org_id cannot be empty".to_string(),
                ));
            }
            if self.workspace_id.is_empty() {
                return Err(SuperpositionError::InvalidConfiguration(
                    "Superposition workspace_id cannot be empty".to_string(),
                ));
            }
        }
        Ok(())
    }
}