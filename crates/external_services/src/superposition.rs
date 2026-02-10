//! Superposition client for dynamic configuration management

/// Type definitions for Superposition integration
pub mod types;

use std::collections::HashMap;

use common_utils::{
    errors::CustomResult,
    id_type
};
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

/// Holds all optional identifiers that can be used to build
/// targeting keys for  experiment resolution.
/// Each field is optional because targeting may be performed
/// at different levels (merchant-level, customer-level, payment-level, etc.).

#[derive(Debug, Clone, Default)]
pub struct TargetingContext {
    /// Unique identifier of the merchant.
    ///
    /// Used for merchant-level targeting.
    pub merchant_id: Option<id_type::MerchantId>,

    /// Unique identifier of the customer.
    ///
    /// Used for customer-specific targeting.
    pub customer_id: Option<id_type::CustomerId>,

    /// Unique identifier of the payment.
    ///
    /// Useful for payment-level targeting.
    pub payment_id: Option<id_type::PaymentId>,

    /// Unique identifier of the business profile.
    ///
    /// Allows profile-scoped configuration and routing logic.
    pub profile_id: Option<id_type::ProfileId>,
}
impl TargetingContext {
    // ---------------------------------------------------------------------
    // Constructors
    // ---------------------------------------------------------------------

    /// Creates an empty `TargetingContext` with no identifiers set.
    ///
    /// Equivalent to calling `TargetingContext::default()`.
    pub fn new() -> Self {
        Self::default()
    }

    // ---------------------------------------------------------------------
    // Builder-style setters (consume self, return updated self)
    // ---------------------------------------------------------------------

    /// Returns a new `TargetingContext` with the given merchant ID set.
    pub fn with_merchant_id(mut self, merchant_id: id_type::MerchantId) -> Self {
        self.merchant_id = Some(merchant_id);
        self
    }

    /// Returns a new `TargetingContext` with the given customer ID set.
    pub fn with_customer_id(mut self, customer_id: id_type::CustomerId) -> Self {
        self.customer_id = Some(customer_id);
        self
    }

    /// Returns a new `TargetingContext` with the given payment ID set.
    pub fn with_payment_id(mut self, payment_id: id_type::PaymentId) -> Self {
        self.payment_id = Some(payment_id);
        self
    }

    /// Returns a new `TargetingContext` with the given profile ID set.
    pub fn with_profile_id(mut self, profile_id: id_type::ProfileId) -> Self {
        self.profile_id = Some(profile_id);
        self
    }

    // ---------------------------------------------------------------------
    // Getters
    // ---------------------------------------------------------------------

    /// Returns a reference to the merchant ID, if present.
    pub fn merchant_id(&self) -> Option<String> {
        self.merchant_id.as_ref().map(|m_id|m_id.get_string_repr().to_owned())
    }

    /// Returns a reference to the customer ID, if present.
    pub fn customer_id(&self) -> Option<String> {
        self.customer_id.as_ref().map(|c_id|c_id.get_string_repr().to_owned())
    }

    /// Returns a reference to the payment ID, if present.
    pub fn payment_id(&self) -> Option<String> {
        self.payment_id.as_ref().map(|pa_id|pa_id.get_string_repr().to_owned())
    }

    /// Returns a reference to the profile ID, if present.
    pub fn profile_id(&self) -> Option<String> {
        self.profile_id.as_ref().map(|p_id|p_id.get_string_repr().to_owned())
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
        targeting_key: Option<&String>
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
        targeting_key : Option<&String>, 
    ) -> CustomResult<T, SuperpositionError>
    where
        open_feature::Client: GetValue<T>,
    {
        let evaluation_context = self.build_evaluation_context(context, targeting_key);
        let type_name = std::any::type_name::<T>();
        router_env::logger::info!("evaluation {:?}", evaluation_context);

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

    /// Get the Superposition key for this config
    const SUPERPOSITION_KEY: &'static str;

    /// Get the default value for this config
    const DEFAULT_VALUE: Self::Output;
    /// Define what targeting key this feature flag uses
    /// Each config implements this to specify its targeting strategy
    fn build_targeting_key(targeting_ctx: &TargetingContext) -> Option<String>;

    /// Fetch config value from Superposition.
    fn fetch(
        superposition_client: &SuperpositionClient,
        context: Option<ConfigContext>,
        targeting_context: &TargetingContext,
    ) -> impl std::future::Future<Output = CustomResult<Self::Output, SuperpositionError>> + Send
    where
        open_feature::Client: GetValue<Self::Output>,
    {
        router_env::logger::info!("in superposition client");
        async move {
            let targeting_key = Self::build_targeting_key(targeting_context);
            router_env::logger::info!("targeting key {:?}",targeting_key);
            match superposition_client
                .get_config_value::<Self::Output>(Self::SUPERPOSITION_KEY, context.as_ref(), targeting_key.as_ref())
                .await
            {
                Ok(value) => {
                    router_env::logger::info!(
                        "Superposition config hit: key='{}', type='{}'",
                        Self::SUPERPOSITION_KEY,
                        std::any::type_name::<Self::Output>()
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
