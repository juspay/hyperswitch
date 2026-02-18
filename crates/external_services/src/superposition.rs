//! Superposition client for dynamic configuration management

/// Type definitions for Superposition integration
pub mod types;

use std::collections::HashMap;

use common_utils::{errors::CustomResult, id_type::TargetingKey};
use error_stack::report;
use masking::ExposeInterface;

pub use self::types::{ConfigContext, SuperpositionClientConfig, SuperpositionError};
use crate::config_metrics;

fn convert_open_feature_value(value: open_feature::Value) -> Result<serde_json::Value, String> {
    match value {
        open_feature::Value::String(s) => Ok(serde_json::Value::String(s)),
        open_feature::Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
        open_feature::Value::Int(n) => Ok(serde_json::Value::Number(serde_json::Number::from(n))),
        open_feature::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| format!("Invalid number: {f}")),
        open_feature::Value::Struct(sv) => Ok(types::JsonValue::try_from(sv)?.into_inner()),
        open_feature::Value::Array(values) => Ok(serde_json::Value::Array(
            values
                .into_iter()
                .map(convert_open_feature_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

/// Trait abstracting the different type-specific client methods
pub trait GetValue<T> {
    /// Get a typed value from the OpenFeature client
    fn get_value(
        &self,
        key: &str,
        context: &open_feature::EvaluationContext,
    ) -> impl std::future::Future<Output = Result<T, open_feature::EvaluationError>> + Send;
}

impl GetValue<bool> for open_feature::Client {
    async fn get_value(
        &self,
        key: &str,
        context: &open_feature::EvaluationContext,
    ) -> Result<bool, open_feature::EvaluationError> {
        self.get_bool_value(key, Some(context), None).await
    }
}

impl GetValue<String> for open_feature::Client {
    async fn get_value(
        &self,
        key: &str,
        context: &open_feature::EvaluationContext,
    ) -> Result<String, open_feature::EvaluationError> {
        self.get_string_value(key, Some(context), None).await
    }
}

impl GetValue<i64> for open_feature::Client {
    async fn get_value(
        &self,
        key: &str,
        context: &open_feature::EvaluationContext,
    ) -> Result<i64, open_feature::EvaluationError> {
        self.get_int_value(key, Some(context), None).await
    }
}

impl GetValue<f64> for open_feature::Client {
    async fn get_value(
        &self,
        key: &str,
        context: &open_feature::EvaluationContext,
    ) -> Result<f64, open_feature::EvaluationError> {
        self.get_float_value(key, Some(context), None).await
    }
}

impl GetValue<serde_json::Value> for open_feature::Client {
    async fn get_value(
        &self,
        key: &str,
        context: &open_feature::EvaluationContext,
    ) -> Result<serde_json::Value, open_feature::EvaluationError> {
        let json_result = self
            .get_struct_value::<types::JsonValue>(key, Some(context), None)
            .await?;
        Ok(json_result.into_inner())
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
            refresh_strategy: superposition_provider::RefreshStrategy::Polling(
                superposition_provider::PollingStrategy {
                    interval: config.polling_interval,
                    timeout: config.request_timeout,
                },
            ),
            experimentation_options: Some(superposition_provider::types::ExperimentationOptions {
                refresh_strategy: superposition_provider::RefreshStrategy::Polling(
                    superposition_provider::PollingStrategy {
                        interval: config.polling_interval,
                        timeout: config.request_timeout,
                    },
                ),
                evaluation_cache: None,
                default_toss: None,
            }),
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
        targeting_key: Option<&String>,
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
            targeting_key: targeting_key.cloned(),
        }
    }

    /// Generic method to get a typed configuration value from Superposition
    ///
    /// # Type Parameters
    /// * `T` - The type of value to retrieve. Supported types: `bool`, `String`, `i64`, `f64`, `serde_json::Value`
    ///
    /// # Arguments
    /// * `key` - The configuration key
    /// * `context` - Optional evaluation context
    ///
    /// # Returns
    /// * `CustomResult<T, SuperpositionError>` - The configuration value or error
    pub async fn get_config_value<T>(
        &self,
        key: &str,
        context: Option<&ConfigContext>,
        targeting_key: Option<&String>,
    ) -> CustomResult<T, SuperpositionError>
    where
        open_feature::Client: GetValue<T>,
    {
        let evaluation_context = self.build_evaluation_context(context, targeting_key);
        let type_name = std::any::type_name::<T>();

        self.client
            .get_value(key, &evaluation_context)
            .await
            .map_err(|e| {
                report!(SuperpositionError::ClientError(format!(
                    "Failed to get {type_name} value for key '{key}': {e:?}"
                )))
            })
    }
}

/// Each config type implements this trait to define how its value should be
/// retrieved from Superposition.
pub trait Config {
    /// The output type of this configuration
    type Output: Default + Clone;

    /// The type used as the targeting key for experiment traffic splitting
    type TargetingKey: TargetingKey + Send + Sync;

    /// Get the Superposition key for this config
    const SUPERPOSITION_KEY: &'static str;

    /// Get the default value for this config
    const DEFAULT_VALUE: Self::Output;

    /// Fetch config value from Superposition.
    fn fetch(
        superposition_client: &SuperpositionClient,
        context: Option<ConfigContext>,
        targeting_key: Option<&Self::TargetingKey>,
    ) -> impl std::future::Future<Output = CustomResult<Self::Output, SuperpositionError>> + Send
    where
        open_feature::Client: GetValue<Self::Output>,
    {
        let targeting_key_str = targeting_key.map(|id| id.targeting_key_value().to_owned());
        async move {
            match superposition_client
                .get_config_value::<Self::Output>(
                    Self::SUPERPOSITION_KEY,
                    context.as_ref(),
                    targeting_key_str.as_ref(),
                )
                .await
            {
                Ok(value) => {
                    router_env::logger::info!(
                        "Superposition config hit: key='{}', type='{}'",
                        Self::SUPERPOSITION_KEY,
                        std::any::type_name::<Self::Output>()
                    );
                    config_metrics::CONFIG_SUPERPOSITION_FETCH.add(
                        1,
                        router_env::metric_attributes!(("config_type", Self::SUPERPOSITION_KEY)),
                    );
                    Ok(value)
                }
                Err(e) => {
                    router_env::logger::warn!(
                        "Superposition config miss: key='{}', type='{}', error='{:?}'",
                        Self::SUPERPOSITION_KEY,
                        std::any::type_name::<Self::Output>(),
                        e
                    );
                    Err(e)
                }
            }
        }
    }
}
