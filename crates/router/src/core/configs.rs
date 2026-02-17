pub mod dimension_config;
pub mod dimension_state;

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::superposition::{self, ConfigContext};

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
/// This function is specifically for DatabaseBackedConfig types and enforces
/// that database fallback is used when superposition fails. It uses the config's
/// `db_key` method to construct the database key from dimensions.
pub async fn fetch_db_with_dimensions<C, M, O, P>(
    storage: &dyn db::StorageInterface,
    superposition_client: Option<&superposition::SuperpositionClient>,
    dimensions: &dimension_state::Dimensions<M, O, P>,
    targeting_key: Option<&C::TargetingKey>,
) -> C::Output
where
    C: DatabaseBackedConfig,
    C::Output: ConfigType,
    M: Send + Sync,
    O: Send + Sync,
    P: Send + Sync,
    open_feature::Client: superposition::GetValue<C::Output>,
{
    let db_key = <C as DatabaseBackedConfig>::db_key(dimensions);
    let context = dimensions.to_superposition_context();

    fetch_db_config::<C>(
        storage,
        superposition_client,
        &db_key,
        context,
        targeting_key,
    )
    .await
}

/// This trait extends external_services::superposition::Config with database-specific metadata
/// and enforces that implementations must provide db_key construction.
pub trait DatabaseBackedConfig: superposition::Config {
    /// The database key suffix for this config
    const KEY: &'static str;

    /// Generate the database key for this config based on dimensions
    fn db_key<M, O, P>(dimensions: &dimension_state::Dimensions<M, O, P>) -> String;
}

/// Fetch configuration value from Superposition with database fallback.
/// This function is specifically for DatabaseBackedConfig types and enforces
/// that database fallback is used when superposition fetch fails.
pub async fn fetch_db_config<C>(
    storage: &dyn db::StorageInterface,
    superposition_client: Option<&superposition::SuperpositionClient>,
    db_key: &str,
    context: Option<ConfigContext>,
    targeting_key: Option<&C::TargetingKey>,
) -> C::Output
where
    C: DatabaseBackedConfig,
    C::Output: ConfigType,
    open_feature::Client: superposition::GetValue<C::Output>,
{
    let default_value = C::DEFAULT_VALUE;
    let config_type = C::KEY;

    let superposition_result = match superposition_client {
        Some(client) => C::fetch(client, context, targeting_key).await,
        None => Err(error_stack::report!(
            superposition::SuperpositionError::ClientError(
                "No superposition client available".to_string()
            )
        )),
    };

    match superposition_result {
        Ok(value) => value,
        Err(_) => {
            router_env::logger::info!("Retrieving config from database for key '{}'", db_key);

            let config_result = storage
                .find_config_by_key_unwrap_or(
                    db_key,
                    Some(default_value.to_config_string().unwrap_or_default()),
                )
                .await;

            match config_result
                .ok()
                .and_then(|config| C::Output::from_config_str(&config.config).ok())
            {
                Some(value) => {
                    metrics::CONFIG_DATABASE_FETCH.add(
                        1,
                        router_env::metric_attributes!(("config_type", config_type)),
                    );
                    value
                }
                None => {
                    router_env::logger::info!("Using default config value for key '{}'", db_key);
                    metrics::CONFIG_DEFAULT_FALLBACK.add(
                        1,
                        router_env::metric_attributes!(("config_type", config_type)),
                    );
                    default_value
                }
            }
        }
    }
}
