//! Superposition client for dynamic configuration management

/// Type definitions for Superposition integration
pub mod types;

use std::collections::HashMap;

use api_models::superposition_proxy::{
    AuditLogResponse, ContextResponse, DefaultConfigResponse, DimensionResponse,
};
pub use aws_smithy_types::DateTime;
use aws_smithy_types::{Document, Number};
use common_utils::{errors::CustomResult, id_type::TargetingKey};
use error_stack::{report, ResultExt};
use hyperswitch_masking::ExposeInterface;
use serde_json::Map;
use superposition_provider::traits::AllFeatureProvider;
pub use superposition_sdk::{
    operation::{
        create_context::builders::CreateContextInputBuilder,
        get_detailed_resolved_config::builders::GetDetailedResolvedConfigInputBuilder,
        list_audit_logs::builders::ListAuditLogsInputBuilder,
        list_contexts::builders::ListContextsInputBuilder,
        list_default_configs::builders::ListDefaultConfigsInputBuilder,
        list_dimensions::builders::ListDimensionsInputBuilder,
    },
    types::{AuditAction, ContextFilterSortOn, DimensionMatchStrategy, SortBy},
};
pub use superposition_types::api::{
    config::ContextPayload as ResolveConfigBody, context::PutRequest as ContextPutRequest,
};

pub use self::types::{ConfigContext, SuperpositionClientConfig, SuperpositionError, ToDocument};
use crate::config_metrics;

/// Convert an `aws_smithy_types::Document` to a `serde_json::Value`.
pub fn document_to_value(doc: Document) -> serde_json::Value {
    match doc {
        Document::Object(obj) => serde_json::Value::Object(
            obj.into_iter()
                .map(|(k, v)| (k, document_to_value(v)))
                .collect(),
        ),
        Document::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(document_to_value).collect())
        }
        Document::Number(num) => match num {
            Number::PosInt(v) => serde_json::Value::Number(serde_json::Number::from(v)),
            Number::NegInt(v) => serde_json::Value::Number(serde_json::Number::from(v)),
            Number::Float(v) => serde_json::Number::from_f64(v)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        },
        Document::String(s) => serde_json::Value::String(s),
        Document::Bool(b) => serde_json::Value::Bool(b),
        Document::Null => serde_json::Value::Null,
    }
}

/// Convert a `serde_json::Value` to an `aws_smithy_types::Document`.
pub fn value_to_document(val: serde_json::Value) -> Document {
    match val {
        serde_json::Value::Null => Document::Null,
        serde_json::Value::Bool(b) => Document::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_u64() {
                Document::Number(Number::PosInt(i))
            } else if let Some(i) = n.as_i64() {
                Document::Number(Number::NegInt(i))
            } else {
                Document::Number(Number::Float(n.as_f64().unwrap_or(0.0)))
            }
        }
        serde_json::Value::String(s) => Document::String(s),
        serde_json::Value::Array(arr) => {
            Document::Array(arr.into_iter().map(value_to_document).collect())
        }
        serde_json::Value::Object(obj) => Document::Object(
            obj.into_iter()
                .map(|(k, v)| (k, value_to_document(v)))
                .collect(),
        ),
    }
}

/// Format a smithy `DateTime` as an RFC3339 string.
pub fn datetime_to_string(dt: &DateTime) -> String {
    dt.fmt(aws_smithy_types::date_time::Format::DateTime)
        .unwrap_or_default()
}

/// Parse an ISO 8601 datetime string into an `aws_smithy_types::DateTime`.
pub fn parse_datetime(s: &str) -> Result<DateTime, String> {
    DateTime::from_str(s, aws_smithy_types::date_time::Format::DateTime).map_err(|e| e.to_string())
}

/// Convert a `HashMap<String, Document>` to a JSON object value.
pub fn doc_map_to_json(map: &HashMap<String, Document>) -> serde_json::Value {
    serde_json::Value::Object(
        map.iter()
            .map(|(k, v)| (k.clone(), document_to_value(v.clone())))
            .collect(),
    )
}

/// Serialize a `DimensionType` to its JSON representation.
pub fn dimension_type_to_value(dt: &superposition_sdk::types::DimensionType) -> serde_json::Value {
    use superposition_sdk::types::DimensionType;
    match dt {
        DimensionType::Regular => serde_json::Value::String("REGULAR".to_string()),
        DimensionType::LocalCohort(s) => serde_json::json!({ "LOCAL_COHORT": s }),
        DimensionType::RemoteCohort(s) => serde_json::json!({ "REMOTE_COHORT": s }),
        _ => serde_json::Value::String("UNKNOWN".to_string()),
    }
}

/// Map a Superposition SDK error to a `SuperpositionError` based on HTTP status.
pub fn map_sdk_error<E: std::fmt::Debug>(
    err: superposition_sdk::error::SdkError<E>,
) -> SuperpositionError {
    let status = err.raw_response().map(|r| r.status().as_u16());
    match status {
        Some(404) => SuperpositionError::NotFound(format!("{err:?}")),
        Some(s) if (400..500).contains(&s) => SuperpositionError::BadRequest(format!("{err:?}")),
        _ => SuperpositionError::ClientError(format!("{err:?}")),
    }
}

/// Convert a Superposition SDK `ContextResponse` into the typed response struct.
pub fn context_response_to_struct(
    ctx: &superposition_sdk::types::ContextResponse,
) -> ContextResponse {
    ContextResponse {
        id: ctx.id().to_owned(),
        value: doc_map_to_json(ctx.value()),
        r#override: doc_map_to_json(ctx.r#override()),
        override_id: ctx.override_id().to_owned(),
        weight: ctx.weight().to_owned(),
        description: ctx.description().to_owned(),
        change_reason: ctx.change_reason().to_owned(),
        created_at: datetime_to_string(ctx.created_at()),
        created_by: ctx.created_by().to_owned(),
        last_modified_at: datetime_to_string(ctx.last_modified_at()),
        last_modified_by: ctx.last_modified_by().to_owned(),
    }
}

/// Convert a Superposition SDK `DefaultConfigResponse` into the typed response struct.
pub fn default_config_response_to_struct(
    cfg: &superposition_sdk::types::DefaultConfigResponse,
) -> DefaultConfigResponse {
    DefaultConfigResponse {
        key: cfg.key().to_owned(),
        value: document_to_value(cfg.value().clone()),
        schema: doc_map_to_json(cfg.schema()),
        description: cfg.description().to_owned(),
        change_reason: cfg.change_reason().to_owned(),
        value_validation_function_name: cfg.value_validation_function_name().map(str::to_owned),
        value_compute_function_name: cfg.value_compute_function_name().map(str::to_owned),
        created_at: datetime_to_string(cfg.created_at()),
        created_by: cfg.created_by().to_owned(),
        last_modified_at: datetime_to_string(cfg.last_modified_at()),
        last_modified_by: cfg.last_modified_by().to_owned(),
    }
}

/// Convert a Superposition SDK `DimensionResponse` into the typed response struct.
pub fn dimension_response_to_struct(
    dim: &superposition_sdk::types::DimensionResponse,
) -> DimensionResponse {
    let dep_graph: Map<String, serde_json::Value> = dim
        .dependency_graph()
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                serde_json::Value::Array(
                    v.iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            )
        })
        .collect();
    DimensionResponse {
        dimension: dim.dimension().to_owned(),
        position: dim.position(),
        schema: doc_map_to_json(dim.schema()),
        value_validation_function_name: dim.value_validation_function_name().map(str::to_owned),
        description: dim.description().to_owned(),
        change_reason: dim.change_reason().to_owned(),
        last_modified_at: datetime_to_string(dim.last_modified_at()),
        last_modified_by: dim.last_modified_by().to_owned(),
        created_at: datetime_to_string(dim.created_at()),
        created_by: dim.created_by().to_owned(),
        dependency_graph: serde_json::Value::Object(dep_graph),
        dimension_type: dimension_type_to_value(dim.dimension_type()),
        value_compute_function_name: dim.value_compute_function_name().map(str::to_owned),
        mandatory: dim.mandatory(),
    }
}

/// Convert a Superposition SDK `AuditLogFull` into the typed response struct.
pub fn audit_log_full_to_struct(log: &superposition_sdk::types::AuditLogFull) -> AuditLogResponse {
    AuditLogResponse {
        id: log.id().to_owned(),
        table_name: log.table_name().to_owned(),
        user_name: log.user_name().to_owned(),
        timestamp: datetime_to_string(log.timestamp()),
        action: log.action().as_str().to_owned(),
        original_data: log.original_data().map(|d| document_to_value(d.clone())),
        new_data: log.new_data().map(|d| document_to_value(d.clone())),
        query: log.query().to_owned(),
    }
}

/// Convert a `ContextPutRequest` into the SDK `ContextPut` type.
pub fn context_put_from_request(
    body: &ContextPutRequest,
) -> CustomResult<superposition_sdk::types::ContextPut, SuperpositionError> {
    let context_json = serde_json::to_value(&body.context).map_err(|e| {
        report!(SuperpositionError::ClientError(format!(
            "Failed to serialize context: {e}"
        )))
    })?;

    let override_json = serde_json::to_value(&body.r#override).map_err(|e| {
        report!(SuperpositionError::ClientError(format!(
            "Failed to serialize override: {e}"
        )))
    })?;

    let mut builder = superposition_sdk::types::ContextPut::builder();

    if let serde_json::Value::Object(ctx_map) = context_json {
        for (k, v) in ctx_map {
            builder = builder.context(k, value_to_document(v));
        }
    }

    if let serde_json::Value::Object(ovr_map) = override_json {
        for (k, v) in ovr_map {
            builder = builder.r#override(k, value_to_document(v));
        }
    }

    if let Some(desc) = &body.description {
        builder = builder.description(String::from(desc));
    }

    builder = builder.change_reason(String::from(&body.change_reason));

    builder.build().map_err(|e| {
        report!(SuperpositionError::ClientError(format!(
            "Failed to build ContextPut: {e:?}"
        )))
    })
}

/// Convert a Superposition SDK `CreateContextOutput` into the shared `ContextResponse` struct.
pub fn create_context_output_to_struct(
    out: &superposition_sdk::operation::create_context::CreateContextOutput,
) -> ContextResponse {
    ContextResponse {
        id: out.id().to_owned(),
        value: doc_map_to_json(out.value()),
        r#override: doc_map_to_json(out.r#override()),
        override_id: out.override_id().to_owned(),
        weight: out.weight().to_owned(),
        description: out.description().to_owned(),
        change_reason: out.change_reason().to_owned(),
        created_at: datetime_to_string(out.created_at()),
        created_by: out.created_by().to_owned(),
        last_modified_at: datetime_to_string(out.last_modified_at()),
        last_modified_by: out.last_modified_by().to_owned(),
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
        let http_source = superposition_provider::data_source::http::HttpDataSource::new(
            superposition_provider::types::SuperpositionOptions::new(
                config.endpoint.clone(),
                token_value.clone(),
                config.org_id.clone(),
                config.workspace_id.clone(),
            ),
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
            .bearer_token(superposition_sdk::config::Token::new(token_value, None))
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
