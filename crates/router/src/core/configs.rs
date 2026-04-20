pub mod dimension_config;
pub mod dimension_state;
use common_utils::errors::CustomResult;
pub use dimension_config::{
    EnableExtendedCardBin, ImplicitCustomerUpdate, RequiresCvv, ShouldCallGsm,
    ShouldEnableMitWithLimitedCardData, ShouldPerformEligibility,
    ShouldStoreEligibilityCheckDataForAuthentication,
};
use error_stack::ResultExt;
pub use external_services::superposition::ConfigContext;
use external_services::superposition;

use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    db,
    routes::{metrics, SessionState},
    services::ApplicationResponse,
    types::{api, transformers::ForeignInto},
};

pub async fn set_config(state: SessionState, config: api::Config) -> RouterResponse<api::Config> {
    let store = state.store.as_ref();
    let config = store
        .insert_config(diesel_models::configs::ConfigNew {
            key: config.key,
            config: config.value,
        })
        .await
        .to_duplicate_response(errors::ApiErrorResponse::DuplicateConfig)
        .attach_printable("Unknown error, while setting config key")?;

    Ok(ApplicationResponse::Json(config.foreign_into()))
}

pub async fn read_config(state: SessionState, key: &str) -> RouterResponse<api::Config> {
    let store = state.store.as_ref();
    let config = store
        .find_config_by_key(key)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ConfigNotFound)?;
    Ok(ApplicationResponse::Json(config.foreign_into()))
}

pub async fn update_config(
    state: SessionState,
    config_update: &api::ConfigUpdate,
) -> RouterResponse<api::Config> {
    let store = state.store.as_ref();
    let config = store
        .update_config_by_key(&config_update.key, config_update.foreign_into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::ConfigNotFound)?;
    Ok(ApplicationResponse::Json(config.foreign_into()))
}

pub async fn config_delete(state: SessionState, key: String) -> RouterResponse<api::Config> {
    let store = state.store.as_ref();
    let config = store
        .delete_config_by_key(&key)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ConfigNotFound)?;
    Ok(ApplicationResponse::Json(config.foreign_into()))
}

/// Trait for types that can be stored and retrieved as a configuration value
pub trait ConfigType: Sized {
    /// Parse the value from database string representation
    fn from_config_str(config_str: &str) -> CustomResult<Self, errors::StorageError>;

    /// Convert the value to a string for database storage
    fn to_config_string(&self) -> CustomResult<String, errors::StorageError>;
}

impl ConfigType for String {
    fn from_config_str(config_str: &str) -> CustomResult<Self, errors::StorageError> {
        Ok(config_str.to_string())
    }

    fn to_config_string(&self) -> CustomResult<String, errors::StorageError> {
        Ok(self.clone())
    }
}

impl ConfigType for bool {
    fn from_config_str(config_str: &str) -> CustomResult<Self, errors::StorageError> {
        config_str
            .parse::<Self>()
            .change_context(errors::StorageError::DeserializationFailed)
    }

    fn to_config_string(&self) -> CustomResult<String, errors::StorageError> {
        Ok(self.to_string())
    }
}

impl ConfigType for i64 {
    fn from_config_str(config_str: &str) -> CustomResult<Self, errors::StorageError> {
        config_str
            .parse::<Self>()
            .change_context(errors::StorageError::DeserializationFailed)
    }

    fn to_config_string(&self) -> CustomResult<String, errors::StorageError> {
        Ok(self.to_string())
    }
}

impl ConfigType for u32 {
    fn from_config_str(config_str: &str) -> CustomResult<Self, errors::StorageError> {
        config_str
            .parse::<Self>()
            .change_context(errors::StorageError::DeserializationFailed)
    }

    fn to_config_string(&self) -> CustomResult<String, errors::StorageError> {
        Ok(self.to_string())
    }
}

impl ConfigType for f64 {
    fn from_config_str(config_str: &str) -> CustomResult<Self, errors::StorageError> {
        config_str
            .parse::<Self>()
            .change_context(errors::StorageError::DeserializationFailed)
    }

    fn to_config_string(&self) -> CustomResult<String, errors::StorageError> {
        Ok(self.to_string())
    }
}

impl ConfigType for serde_json::Value {
    fn from_config_str(config_str: &str) -> CustomResult<Self, errors::StorageError> {
        serde_json::from_str(config_str).change_context(errors::StorageError::DeserializationFailed)
    }

    fn to_config_string(&self) -> CustomResult<String, errors::StorageError> {
        serde_json::to_string(self).change_context(errors::StorageError::SerializationFailed)
    }
}

/// Fetch configuration value from Superposition with database fallback using dimension-aware key.
/// This function accepts any type that implements DimensionsBase (including type aliases).
/// This allows configs to be used with pre-defined dimension type aliases like DimensionsWithProcessorAndProviderMerchantId or DimensionsWithProcessorAndProviderMerchantIdAndProfileId.
pub async fn fetch_db_config_for_dimensions<C>(
    storage: &dyn db::StorageInterface,
    superposition_client: &superposition::SuperpositionClient,
    dimensions: &impl dimension_state::DimensionsBase,
    targeting_key: Option<&C::TargetingKey>,
) -> C::Output
where
    C: DatabaseBackedConfig,
    C::Output: ConfigType,
    open_feature::Client: superposition::GetValue<C::Output>,
{
    let db_key = <C as DatabaseBackedConfig>::db_key(dimensions);
    let context = dimensions.to_superposition_context();

    fetch_db_config::<C>(
        storage,
        superposition_client,
        db_key.as_deref(),
        context,
        targeting_key,
    )
    .await
}

/// This trait extends external_services::superposition::Config with database-specific metadata
/// and enforces that implementations must provide db_key construction.
pub trait DatabaseBackedConfig: superposition::Config {
    /// The database key prefix/suffix for this config
    const KEY: &'static str;

    /// Generate the database key for this config based on dimensions
    fn db_key(dimensions: &impl dimension_state::DimensionsBase) -> Option<String>;

    /// Parse the raw database config string into the output type.
    /// Override this for configs whose DB format differs from the Output type
    /// (e.g. a list stored in DB that must be converted to a bool using context).
    fn parse_db_config(
        config_str: &str,
        _context: Option<&ConfigContext>,
    ) -> Option<Self::Output>
    where
        Self::Output: ConfigType,
    {
        Self::Output::from_config_str(config_str).ok()
    }
}

/// Fetch configuration value from Superposition with database fallback.
/// This function is specifically for DatabaseBackedConfig types and enforces
/// that database fallback is used when superposition fetch fails.
pub async fn fetch_db_config<C>(
    storage: &dyn db::StorageInterface,
    superposition_client: &superposition::SuperpositionClient,
    db_key: Option<&str>,
    context: Option<ConfigContext>,
    targeting_key: Option<&C::TargetingKey>,
) -> C::Output
where
    C: DatabaseBackedConfig,
    C::Output: ConfigType,
    open_feature::Client: superposition::GetValue<C::Output>,
{
    let config_type = C::KEY;
    let default_value = C::default_value();

    let superposition_result = C::fetch(superposition_client, context.as_ref(), targeting_key).await;

    let resolved_value = match superposition_result {
        Ok(value) => {
            router_env::logger::info!(
                config_key = %config_type,
                source = "superposition",
                value = %value.to_config_string().unwrap_or_default(),
                "Config resolved from superposition"
            );
            value
        }
        Err(_) => match db_key {
            Some(db_key) => {
                router_env::logger::info!(
                    "Retrieving config from database for key '{}'",
                    config_type
                );

                let config_result = storage.find_config_by_key(db_key).await;

                match config_result
                    .ok()
                    .and_then(|config| C::parse_db_config(&config.config, context.as_ref()))
                {
                    Some(value) => {
                        router_env::logger::info!(
                            config_key = %config_type,
                            db_key = %db_key,
                            source = "database",
                            value = %value.to_config_string().unwrap_or_default(),
                            "Config resolved from database"
                        );
                        metrics::CONFIG_DATABASE_FETCH.add(
                            1,
                            router_env::metric_attributes!(("config_type", config_type)),
                        );
                        value
                    }
                    None => {
                        router_env::logger::info!(
                            "Using default config value for key '{}'",
                            config_type
                        );
                        metrics::CONFIG_DEFAULT_FALLBACK.add(
                            1,
                            router_env::metric_attributes!(("config_type", config_type)),
                        );
                        default_value
                    }
                }
            }
            None => {
                router_env::logger::info!(
                    "No database key provided for config '{}', using default value",
                    config_type
                );
                metrics::CONFIG_DEFAULT_FALLBACK.add(
                    1,
                    router_env::metric_attributes!(("config_type", config_type)),
                );
                default_value
            }
        },
    };
    resolved_value
}

/// Fetch object-type config with JSON-to-Type conversion.
/// Used when Config Output is serde_json::Value but caller wants a specific type.
pub async fn fetch_db_config_object<C, T>(
    storage: &dyn db::StorageInterface,
    superposition_client: &superposition::SuperpositionClient,
    db_key: Option<&str>,
    context: Option<ConfigContext>,
    targeting_key: Option<&C::TargetingKey>,
) -> T
where
    C: DatabaseBackedConfig<Output = serde_json::Value>,
    T: for<'de> serde::Deserialize<'de> + Default,
    open_feature::Client: superposition::GetValue<serde_json::Value>,
{
    let json_value = fetch_db_config::<C>(
        storage,
        superposition_client,
        db_key,
        context,
        targeting_key,
    )
    .await;
    let config_type = C::KEY;

    serde_json::from_value(json_value).unwrap_or_else(|e| {
        router_env::logger::error!(
            "Failed to deserialize {}: {:?}, using default",
            stringify!(T),
            e
        );
        metrics::CONFIG_DEFAULT_FALLBACK.add(
            1,
            router_env::metric_attributes!(("config_type", config_type)),
        );
        T::default()
    })
}

/// Fetch dimension-aware object-type config with JSON deserialization.
pub async fn fetch_db_config_for_objects<C, T>(
    storage: &dyn db::StorageInterface,
    superposition_client: &superposition::SuperpositionClient,
    dimensions: &impl dimension_state::DimensionsBase,
    targeting_key: Option<&C::TargetingKey>,
) -> T
where
    C: DatabaseBackedConfig<Output = serde_json::Value>,
    T: for<'de> serde::Deserialize<'de> + Default,
    open_feature::Client: superposition::GetValue<serde_json::Value>,
{
    let db_key = <C as DatabaseBackedConfig>::db_key(dimensions);
    let context = dimensions.to_superposition_context();

    fetch_db_config_object::<C, T>(
        storage,
        superposition_client,
        db_key.as_deref(),
        context,
        targeting_key,
    )
    .await
}

/// Fetch dimension-aware string-enum config with String-to-Enum parsing.
/// Used when Config Output is String but caller wants a specific enum type.
pub async fn fetch_db_config_for_string_enum<C, T>(
    storage: &dyn db::StorageInterface,
    superposition_client: &superposition::SuperpositionClient,
    dimensions: &impl dimension_state::DimensionsBase,
    targeting_key: Option<&C::TargetingKey>,
) -> T
where
    C: DatabaseBackedConfig<Output = String>,
    T: std::str::FromStr + Default,
    open_feature::Client: superposition::GetValue<String>,
{
    let db_key = <C as DatabaseBackedConfig>::db_key(dimensions);
    let context = dimensions.to_superposition_context();

    let s = fetch_db_config::<C>(
        storage,
        superposition_client,
        db_key.as_deref(),
        context,
        targeting_key,
    )
    .await;

    let config_type = C::KEY;
    s.parse::<T>().unwrap_or_else(|_| {
        router_env::logger::error!(
            "Failed to parse string enum for config '{}', using default",
            config_type
        );
        metrics::CONFIG_DEFAULT_FALLBACK.add(
            1,
            router_env::metric_attributes!(("config_type", config_type)),
        );
        T::default()
    })
}
