//! Superposition client for dynamic configuration management

/// Type definitions for Superposition integration
pub mod types;

use std::collections::HashMap;

use common_utils::errors::CustomResult;
use error_stack::report;
use masking::ExposeInterface;

pub use self::types::{ConfigContext, SuperpositionClientConfig, SuperpositionError};

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
            .get_struct_value::<types::JsonValue>(key, Some(&evaluation_context), None)
            .await
            .map_err(|e| {
                report!(SuperpositionError::ClientError(format!(
                    "Failed to get object value for key '{key}': {e:?}"
                )))
            })?;

        Ok(json_result.into_inner())
    }
}
