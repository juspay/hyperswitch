//! Superposition client for dynamic configuration management

/// Type definitions for Superposition integration
pub mod types;

use std::collections::HashMap;

use aws_smithy_types::Document;
use common_utils::{errors::CustomResult, id_type::TargetingKey};
use error_stack::{report, ResultExt};
use hyperswitch_masking::ExposeInterface;
use serde_json::Map;
use superposition_provider::traits::AllFeatureProvider;

pub use self::types::{ConfigContext, SuperpositionClientConfig, SuperpositionError, ToDocument};
use crate::config_metrics;

tokio::task_local! {
    /// Per-request `x-user` header payload forwarded to Superposition when the
    /// Internal-scheme auth bypass is active. Set by proxy handlers, consumed
    /// by [`InternalAuthInterceptor`].
    pub static SUPERPOSITION_X_USER: String;
}

/// Smithy interceptor that rewrites the SDK's default `Authorization: Bearer ...`
/// header into the `Authorization: Internal <token>` scheme expected by
/// Superposition's `auth_n.rs` bypass branch, and forwards an `x-user`
/// identity header. Identity precedence: request-scoped task-local
/// [`SUPERPOSITION_X_USER`] > static fallback supplied at construction. The
/// static fallback is used by background flows (e.g. provider polling) that
/// have no per-request context.
#[derive(Debug, Clone)]
pub struct InternalAuthInterceptor {
    token: hyperswitch_masking::Secret<String>,
    static_x_user: Option<String>,
}

impl InternalAuthInterceptor {
    /// Create a new interceptor with the static service token. When no
    /// task-local identity is set, no `x-user` header is sent.
    pub fn new(token: hyperswitch_masking::Secret<String>) -> Self {
        Self {
            token,
            static_x_user: None,
        }
    }

    /// Create an interceptor that falls back to the given `x-user` payload
    /// when the task-local identity is not set.
    pub fn with_static_x_user(
        token: hyperswitch_masking::Secret<String>,
        static_x_user: String,
    ) -> Self {
        Self {
            token,
            static_x_user: Some(static_x_user),
        }
    }
}

impl superposition_sdk::config::Intercept for InternalAuthInterceptor {
    fn name(&self) -> &'static str {
        "SuperpositionInternalAuthInterceptor"
    }

    fn modify_before_transmit(
        &self,
        context: &mut superposition_sdk::config::interceptors::BeforeTransmitInterceptorContextMut<
            '_,
        >,
        _runtime_components: &superposition_sdk::config::RuntimeComponents,
        _cfg: &mut superposition_sdk::config::ConfigBag,
    ) -> Result<(), aws_smithy_runtime_api::box_error::BoxError> {
        let headers = context.request_mut().headers_mut();

        let internal_value = format!("Internal {}", self.token.clone().expose());
        headers.insert("authorization", internal_value);

        let x_user = SUPERPOSITION_X_USER
            .try_with(|payload| payload.clone())
            .ok()
            .or_else(|| self.static_x_user.clone());

        if let Some(user_payload) = x_user {
            headers.insert("x-user", user_payload);
        }

        Ok(())
    }
}

/// Build the `x-user` payload used when polling Superposition from a
/// background flow. The shape matches Superposition's `auth_n.rs` `User`
/// deserializer for the Internal-scheme bypass.
fn build_service_x_user_payload(service_name: &str) -> String {
    serde_json::json!({
        "email": format!("{service_name}@hyperswitch.internal"),
        "username": service_name,
    })
    .to_string()
}

/// Custom Superposition data source backed by a `superposition_sdk::Client`
/// built with [`InternalAuthInterceptor`]. Replaces
/// [`superposition_provider::data_source::http::HttpDataSource`] when the
/// upstream Superposition service runs with OIDC enabled, since that source
/// hardcodes Bearer-token auth with no way to inject extra headers.
#[allow(missing_debug_implementations)]
pub struct HyperswitchHttpDataSource {
    options: superposition_provider::types::SuperpositionOptions,
    client: superposition_sdk::Client,
}

impl HyperswitchHttpDataSource {
    /// Build a data source whose SDK client routes through the supplied
    /// interceptor. The interceptor owns the auth-header rewrite.
    pub fn new(
        options: superposition_provider::types::SuperpositionOptions,
        interceptor: InternalAuthInterceptor,
    ) -> Self {
        let sdk_config = superposition_sdk::Config::builder()
            .endpoint_url(&options.endpoint)
            .bearer_token(superposition_sdk::config::Token::new(
                options.token.clone(),
                None,
            ))
            .behavior_version_latest()
            .interceptor(interceptor)
            .build();

        let client = superposition_sdk::Client::from_conf(sdk_config);
        Self { options, client }
    }

    async fn fetch_experiments_with_filters(
        &self,
        context: Option<serde_json::Map<String, serde_json::Value>>,
        prefix_filter: Option<Vec<String>>,
        if_modified_since: Option<chrono::DateTime<chrono::Utc>>,
        dimension_match_strategy: Option<superposition_sdk::types::DimensionMatchStrategy>,
    ) -> superposition_provider::types::Result<
        superposition_provider::data_source::FetchResponse<superposition_provider::ExperimentData>,
    > {
        let mut builder = self
            .client
            .get_experiment_config()
            .workspace_id(&self.options.workspace_id)
            .org_id(&self.options.org_id);

        if let Some(modified_since) = if_modified_since
            .and_then(|t| t.timestamp_nanos_opt())
            .and_then(|t| aws_smithy_types::DateTime::from_nanos(t.into()).ok())
        {
            builder = builder.if_modified_since(modified_since);
        }

        if let Some(context) = context {
            if !context.is_empty() {
                let context_map = superposition_provider::conversions::map_to_hashmap(context);
                builder = builder.set_context(Some(context_map));
            }
        }

        if let Some(prefixes) = prefix_filter {
            if !prefixes.is_empty() {
                builder = builder.set_prefix(Some(prefixes));
            }
        }

        if let Some(filter) = dimension_match_strategy {
            builder = builder.dimension_match_strategy(filter);
        }

        let result = builder.send().await;
        match result {
            Ok(res) => {
                use chrono::TimeZone as _;
                let modified_at = chrono::Utc.timestamp_nanos(res.last_modified.as_nanos() as i64);
                superposition_provider::utils::ConversionUtils::convert_experiment_config_response(
                    res,
                )
                .map(|d| superposition_provider::ExperimentData {
                    data: d,
                    fetched_at: modified_at,
                })
                .map(superposition_provider::data_source::FetchResponse::Data)
            }
            Err(superposition_sdk::error::SdkError::ResponseError(r))
                if r.raw().status().as_u16() == 304 =>
            {
                Ok(superposition_provider::data_source::FetchResponse::NotModified)
            }
            Err(e) => Err(
                superposition_provider::types::SuperpositionError::NetworkError(format!(
                    "Failed to list experiments: {e}"
                )),
            ),
        }
    }
}

#[async_trait::async_trait]
impl superposition_provider::data_source::SuperpositionDataSource for HyperswitchHttpDataSource {
    async fn fetch_filtered_config(
        &self,
        context: Option<serde_json::Map<String, serde_json::Value>>,
        prefix_filter: Option<Vec<String>>,
        if_modified_since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> superposition_provider::types::Result<
        superposition_provider::data_source::FetchResponse<superposition_provider::ConfigData>,
    > {
        let mut builder = self
            .client
            .get_config()
            .workspace_id(&self.options.workspace_id)
            .org_id(&self.options.org_id);

        if let Some(modified_since) = if_modified_since
            .and_then(|t| t.timestamp_nanos_opt())
            .and_then(|t| aws_smithy_types::DateTime::from_nanos(t.into()).ok())
        {
            builder = builder.if_modified_since(modified_since);
        }

        if let Some(context) = context {
            if !context.is_empty() {
                let context_map = superposition_provider::conversions::map_to_hashmap(context);
                builder = builder.set_context(Some(context_map));
            }
        }

        if let Some(prefixes) = prefix_filter {
            if !prefixes.is_empty() {
                builder = builder.set_prefix(Some(prefixes));
            }
        }

        let result = builder.send().await;
        match result {
            Ok(res) => {
                use chrono::TimeZone as _;
                let modified_at = chrono::Utc.timestamp_nanos(res.last_modified.as_nanos() as i64);
                superposition_provider::utils::ConversionUtils::convert_get_config_response(res)
                    .map(|d| superposition_provider::ConfigData {
                        data: d,
                        fetched_at: modified_at,
                    })
                    .map(superposition_provider::data_source::FetchResponse::Data)
            }
            Err(superposition_sdk::error::SdkError::ResponseError(r))
                if r.raw().status().as_u16() == 304 =>
            {
                Ok(superposition_provider::data_source::FetchResponse::NotModified)
            }
            Err(e) => Err(
                superposition_provider::types::SuperpositionError::NetworkError(format!(
                    "Failed to fetch config: {e}"
                )),
            ),
        }
    }

    async fn fetch_active_experiments(
        &self,
        if_modified_since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> superposition_provider::types::Result<
        superposition_provider::data_source::FetchResponse<superposition_provider::ExperimentData>,
    > {
        self.fetch_experiments_with_filters(None, None, if_modified_since, None)
            .await
    }

    async fn fetch_candidate_active_experiments(
        &self,
        context: Option<serde_json::Map<String, serde_json::Value>>,
        prefix_filter: Option<Vec<String>>,
        if_modified_since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> superposition_provider::types::Result<
        superposition_provider::data_source::FetchResponse<superposition_provider::ExperimentData>,
    > {
        self.fetch_experiments_with_filters(
            context,
            prefix_filter,
            if_modified_since,
            Some(superposition_sdk::types::DimensionMatchStrategy::Exact),
        )
        .await
    }

    async fn fetch_matching_active_experiments(
        &self,
        context: Option<serde_json::Map<String, serde_json::Value>>,
        prefix_filter: Option<Vec<String>>,
        if_modified_since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> superposition_provider::types::Result<
        superposition_provider::data_source::FetchResponse<superposition_provider::ExperimentData>,
    > {
        self.fetch_experiments_with_filters(
            context,
            prefix_filter,
            if_modified_since,
            Some(superposition_sdk::types::DimensionMatchStrategy::Subset),
        )
        .await
    }

    fn supports_experiments(&self) -> bool {
        true
    }

    async fn close(&self) -> superposition_provider::types::Result<()> {
        Ok(())
    }
}

/// Generate a default change reason from the config key
fn generate_change_reason(key: &str) -> String {
    format!("Updating {key} configuration")
}

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

impl GetValue<u32> for open_feature::Client {
    async fn get_value(
        &self,
        key: &str,
        context: &open_feature::EvaluationContext,
    ) -> Result<u32, open_feature::EvaluationError> {
        let value = self.get_int_value(key, Some(context), None).await?;
        u32::try_from(value).map_err(|err| {
            open_feature::EvaluationError::builder()
                .code(open_feature::EvaluationErrorCode::TypeMismatch)
                .message(err.to_string())
                .build()
        })
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
    /// OpenFeature client for reading configs
    client: open_feature::Client,
    /// Provider for Superposition (using LocalResolutionProvider for fallback support)
    provider: superposition_provider::local_provider::LocalResolutionProvider,
    /// SDK client for writing configs (create/update operations)
    sdk_client: superposition_sdk::Client,
    /// Organization ID for write operations
    org_id: String,
    /// Workspace ID for write operations
    workspace_id: String,
    /// Name of the service performing writes, attached to audit metadata
    service_name: &'static str,
}

impl SuperpositionClient {
    /// Create a new Superposition client.
    pub async fn new(
        config: SuperpositionClientConfig,
        service_name: &'static str,
    ) -> CustomResult<Self, SuperpositionError> {
        let token_value = config.token.expose();

        let refresh_strategy = superposition_provider::RefreshStrategy::Polling(
            superposition_provider::PollingStrategy {
                interval: config.polling_interval,
                timeout: config.request_timeout,
            },
        );

        // --- Build HTTP (primary) data source ---
        // We use a custom data source instead of
        // `superposition_provider::data_source::http::HttpDataSource` so the
        // polling client routes through `InternalAuthInterceptor` and works
        // when Superposition has OIDC enabled (Bearer auth is rejected; the
        // Internal scheme with an `x-user` payload is the supported
        // service-account bypass).
        let polling_interceptor = InternalAuthInterceptor::with_static_x_user(
            hyperswitch_masking::Secret::new(token_value.clone()),
            build_service_x_user_payload(service_name),
        );
        let http_source = HyperswitchHttpDataSource::new(
            superposition_provider::types::SuperpositionOptions::new(
                config.endpoint.clone(),
                token_value.clone(),
                config.org_id.clone(),
                config.workspace_id.clone(),
            ),
            polling_interceptor,
        );

        // --- Build File (fallback) data source if backup_file_path is configured ---
        let fallback_source: Option<
            Box<dyn superposition_provider::data_source::SuperpositionDataSource>,
        > = match &config.backup_file_path {
            Some(backup_path) => {
                router_env::logger::info!(
                    "Configuring Superposition file fallback: path={:?}",
                    backup_path
                );
                match superposition_provider::data_source::file::FileDataSource::new(
                    backup_path.clone(),
                ) {
                    Ok(source) => {
                        let boxed: Box<
                            dyn superposition_provider::data_source::SuperpositionDataSource,
                        > = Box::new(source);
                        Some(boxed)
                    }
                    Err(e) => {
                        router_env::logger::warn!(
                            "Failed to create Superposition file fallback source: {e}"
                        );
                        None
                    }
                }
            }
            None => None,
        };

        // --- Build LocalResolutionProvider with HTTP primary and optional file fallback ---
        let provider = superposition_provider::local_provider::LocalResolutionProvider::new(
            Box::new(http_source),
            fallback_source,
            refresh_strategy,
        );

        // Initialize provider - this will try HTTP first, then fallback to file if configured
        provider
            .init(open_feature::EvaluationContext::default())
            .await
            .change_context(SuperpositionError::ClientInitError(
                "Failed to initialize Superposition provider".to_string(),
            ))
            .attach_printable(
                "Both HTTP and file fallback (if configured) initialization failed",
            )?;

        // Initialize OpenFeature API and set provider
        let mut api = open_feature::OpenFeature::singleton_mut().await;
        api.set_provider(provider.clone()).await;

        // Create client
        let client = api.create_client();

        router_env::logger::info!("Superposition client initialized successfully");

        // Initialize SDK client for write operations
        // Use the same token_value extracted earlier for the provider
        let sdk_config = superposition_sdk::Config::builder()
            .endpoint_url(config.endpoint.clone())
            .bearer_token(superposition_sdk::config::Token::new(
                token_value.clone(),
                None,
            ))
            .interceptor(InternalAuthInterceptor::new(
                hyperswitch_masking::Secret::new(token_value),
            ))
            .build();

        let sdk_client = superposition_sdk::Client::from_conf(sdk_config);

        router_env::logger::info!("Superposition SDK client initialized successfully");

        Ok(Self {
            client,
            sdk_client,
            provider,
            org_id: config.org_id,
            workspace_id: config.workspace_id,
            service_name,
        })
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

    /// Resolve full configuration from Superposition
    ///
    /// # Arguments
    /// * `context` - Evaluation context
    ///
    /// # Returns
    /// * `CustomResult<Map<String, serde_json::Value>, SuperpositionError>` - The full configuration or error
    pub async fn resolve_full_config(
        &self,
        context: Option<&ConfigContext>,
        targeting_key: Option<&String>,
    ) -> CustomResult<Map<String, serde_json::Value>, SuperpositionError> {
        let evaluation_context = self.build_evaluation_context(context, targeting_key);
        self.provider
            .resolve_all_features(evaluation_context)
            .await
            .map_err(|e| {
                report!(SuperpositionError::ProviderError(format!(
                    "Failed to resolve full config: {e:?}"
                )))
            })
    }

    /// Get cached configuration from Superposition
    ///
    /// # Arguments
    /// * `prefix_filter` - Optional prefix filter for configuration keys
    /// * `dimension_filter` - Optional dimension filter for configuration values
    ///
    /// # Returns
    /// * `CustomResult<Config, SuperpositionError>` - The cached configuration or error
    pub async fn get_cached_config(
        &self,
        prefix_filter: Option<Vec<String>>,
        dimension_filter: Option<Map<String, serde_json::Value>>,
    ) -> CustomResult<superposition_types::Config, SuperpositionError> {
        use superposition_provider::data_source::SuperpositionDataSource;
        let response = self
            .provider
            .fetch_filtered_config(dimension_filter, prefix_filter, None)
            .await
            .map_err(|e| {
                report!(SuperpositionError::ProviderError(format!(
                    "Failed to get cached config: {e:?}"
                )))
            })?;
        match response {
            superposition_provider::data_source::FetchResponse::Data(data) => Ok(data.data),
            superposition_provider::data_source::FetchResponse::NotModified => {
                Err(report!(SuperpositionError::ProviderError(
                    "Config not modified but no data available".to_string()
                )))
            }
        }
    }

    /// Generic method to set a configuration value in Superposition
    ///
    /// # Type Parameters
    /// * `T` - A type implementing `WritableConfig` that defines the config key and input type
    ///
    /// # Arguments
    /// * `value` - The value to write
    /// * `context` - The context (dimensions) for this config
    ///
    /// # Returns
    /// * `CustomResult<(), SuperpositionError>` - Success or error
    pub async fn set_config_value<T: WritableConfig>(
        &self,
        value: &T::Input,
        context: &ConfigContext,
    ) -> CustomResult<(), SuperpositionError> {
        let mut builder = superposition_sdk::types::ContextPut::builder();

        // Add context entries (dimensions)
        for (key, val) in &context.values {
            builder = builder.context(key, Document::String(val.clone()));
        }

        let change_reason = generate_change_reason(T::SUPERPOSITION_KEY);

        builder = builder
            .r#override(T::SUPERPOSITION_KEY, value.to_document())
            .change_reason(change_reason)
            .description(format!(
                "[{}] Config update for {}",
                self.service_name,
                T::SUPERPOSITION_KEY
            ));

        let context_put = builder.build().map_err(|e| {
            report!(SuperpositionError::ClientError(format!(
                "Failed to build ContextPut: {e:?}"
            )))
        })?;

        // Call create_context API
        let response = self
            .sdk_client
            .create_context()
            .workspace_id(self.workspace_id.clone())
            .org_id(self.org_id.clone())
            .request(context_put)
            .send()
            .await;

        response.change_context(SuperpositionError::ClientError(format!(
            "Failed to set {} config",
            T::SUPERPOSITION_KEY
        )))?;

        router_env::logger::info!("Set {} config successfully", T::SUPERPOSITION_KEY);

        Ok(())
    }

    /// Return a reference to the underlying Superposition SDK client.
    pub fn superposition_sdk_client(&self) -> &superposition_sdk::Client {
        &self.sdk_client
    }
}

/// Trait for configs that can be written to Superposition.
/// This is separate from the Config trait - a config type can implement
/// one or both traits depending on whether it supports read and/or write operations.
pub trait WritableConfig {
    /// The type of value to write (input type)
    type Input: ToDocument;

    /// The Superposition key for this config
    const SUPERPOSITION_KEY: &'static str;
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
    /// Default implementation uses `Default::default()`, can be overridden for custom defaults
    fn default_value() -> Self::Output {
        Self::Output::default()
    }

    /// Fetch config value from Superposition.
    fn fetch(
        superposition_client: &SuperpositionClient,
        context: Option<&ConfigContext>,
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
                    context,
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
