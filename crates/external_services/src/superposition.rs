//! Superposition client for dynamic configuration management

use std::collections::HashMap;

use common_utils::{errors::CustomResult, fp_utils::when};
use masking::{ExposeInterface, Secret};
use superposition_provider;

/// Wrapper type for JSON values from Superposition
#[derive(Debug, Clone)]
struct JsonValue(serde_json::Value);

impl TryFrom<open_feature::StructValue> for JsonValue {
    type Error = String;

    fn try_from(sv: open_feature::StructValue) -> Result<Self, Self::Error> {
        let capacity = sv.fields.len();
        sv.fields
            .into_iter()
            .try_fold(
                serde_json::Map::with_capacity(capacity),
                |mut map, (k, v)| {
                    let value = convert_open_feature_value(v)?;
                    map.insert(k, value);
                    Ok(map)
                },
            )
            .map(|map| Self(serde_json::Value::Object(map)))
    }
}

fn convert_open_feature_value(v: open_feature::Value) -> Result<serde_json::Value, String> {
    match v {
        open_feature::Value::String(s) => Ok(serde_json::Value::String(s)),
        open_feature::Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
        open_feature::Value::Int(n) => Ok(serde_json::Value::Number(serde_json::Number::from(n))),
        open_feature::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| format!("Invalid number: {f}")),
        open_feature::Value::Struct(sv) => Ok(JsonValue::try_from(sv)?.0),
        open_feature::Value::Array(values) => Ok(serde_json::Value::Array(
            values
                .into_iter()
                .map(convert_open_feature_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

/// Configuration for Superposition integration
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
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
    pub polling_interval: u64,
    /// Request timeout in seconds for Superposition API calls (None = no timeout)
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
            polling_interval: 15,
            request_timeout: None,
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
    values: HashMap<String, String>,
}

impl SuperpositionClientConfig {
    /// Validate the Superposition configuration
    pub fn validate(&self) -> Result<(), SuperpositionError> {
        when(self.enabled, || {
            when(self.endpoint.is_empty(), || {
                Err(SuperpositionError::InvalidConfiguration(
                    "Superposition endpoint cannot be empty".to_string(),
                ))
            })
            .and_then(|_| {
                when(self.token.clone().expose().is_empty(), || {
                    Err(SuperpositionError::InvalidConfiguration(
                        "Superposition token cannot be empty".to_string(),
                    ))
                })
            })
            .and_then(|_| {
                when(self.org_id.is_empty(), || {
                    Err(SuperpositionError::InvalidConfiguration(
                        "Superposition org_id cannot be empty".to_string(),
                    ))
                })
            })
            .and_then(|_| {
                when(self.workspace_id.is_empty(), || {
                    Err(SuperpositionError::InvalidConfiguration(
                        "Superposition workspace_id cannot be empty".to_string(),
                    ))
                })
            })
        })
        .unwrap_or(Ok(()))
    }
}

impl ConfigContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a key-value pair to the context. Replaces existing value if key exists.
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }
}

/// Superposition client wrapper
// Debug trait cannot be derived because open_feature::Client doesn't implement Debug
#[allow(missing_debug_implementations)]
pub struct SuperpositionClient {
    client: open_feature::Client,
}

impl SuperpositionClient {
    /// Create a new Superposition client
    pub async fn new(config: SuperpositionClientConfig) -> CustomResult<Self, SuperpositionError> {
        let provider_options = superposition_provider::SuperpositionProviderOptions {
            endpoint: config.endpoint.clone(),
            token: config.token.expose(),
            org_id: config.org_id.clone(),
            workspace_id: config.workspace_id.clone(),
            fallback_config: None,
            evaluation_cache: None,
            refresh_strategy: superposition_provider::RefreshStrategy::Polling(superposition_provider::PollingStrategy {
                interval: config.polling_interval,
                timeout: config.request_timeout,
            }),
            experimentation_options: None,
        };

        // Create provider and set up OpenFeature
        let provider = superposition_provider::SuperpositionProvider::new(provider_options);

        // Initialize OpenFeature API and set provider
        let mut api = open_feature::OpenFeature::singleton_mut().await;
        api.set_provider(provider).await;

        // Create client
        let client = api.create_client();

        router_env::logger::info!("Superposition client initialized successfully");

        Ok(Self { client })
    }

    /// Build evaluation context for Superposition requests
    fn build_evaluation_context(
        &self,
        context: Option<&ConfigContext>,
    ) -> open_feature::EvaluationContext {
        open_feature::EvaluationContext {
            custom_fields: context.map_or(HashMap::new(), |ctx| {
                ctx.values
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            open_feature::EvaluationContextFieldValue::String(v.clone()),
                        )
                    })
                    .collect()
            }),
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
                report!(SuperpositionError::ClientError(format!(
                    "Failed to get bool value for key '{key}': {e:?}"
                )))
            })
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
                report!(SuperpositionError::ClientError(format!(
                    "Failed to get string value for key '{key}': {e:?}"
                )))
            })
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
                report!(SuperpositionError::ClientError(format!(
                    "Failed to get int value for key '{key}': {e:?}"
                )))
            })
    }

    /// Get a float configuration value from Superposition
    pub async fn get_float_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> CustomResult<f64, SuperpositionError> {
        let evaluation_context = self.build_evaluation_context(context);

        self.client
            .get_float_value(key, Some(&evaluation_context), None)
            .await
            .map_err(|e| {
                report!(SuperpositionError::ClientError(format!(
                    "Failed to get float value for key '{key}': {e:?}"
                )))
            })
    }

    /// Get an object configuration value from Superposition
    pub async fn get_object_value(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
    ) -> CustomResult<serde_json::Value, SuperpositionError> {
        let evaluation_context = self.build_evaluation_context(context);

        let json_result = self
            .client
            .get_struct_value::<JsonValue>(key, Some(&evaluation_context), None)
            .await
            .map_err(|e| {
                report!(SuperpositionError::ClientError(format!(
                    "Failed to get object value for key '{key}': {e:?}"
                )))
            })?;

        Ok(json_result.0)
    }
}

#[cfg(feature = "superposition")]
#[async_trait::async_trait]
impl hyperswitch_interfaces::secrets_interface::secret_handler::SecretsHandler
    for SuperpositionClientConfig
{
    async fn convert_to_raw_secret(
        value: hyperswitch_interfaces::secrets_interface::secret_state::SecretStateContainer<
            Self,
            hyperswitch_interfaces::secrets_interface::secret_state::SecuredSecret,
        >,
        secret_management_client: &dyn hyperswitch_interfaces::secrets_interface::SecretManagementInterface,
    ) -> CustomResult<
        hyperswitch_interfaces::secrets_interface::secret_state::SecretStateContainer<
            Self,
            hyperswitch_interfaces::secrets_interface::secret_state::RawSecret,
        >,
        hyperswitch_interfaces::secrets_interface::SecretsManagementError,
    > {
        let superposition_config = value.get_inner();
        let token = if superposition_config.enabled {
            secret_management_client
                .get_secret(superposition_config.token.clone())
                .await?
        } else {
            superposition_config.token.clone()
        };

        Ok(value.transition_state(|superposition_config| Self {
            token,
            ..superposition_config
        }))
    }
}
