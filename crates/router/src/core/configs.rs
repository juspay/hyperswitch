pub mod dimension_config;
pub mod dimension_state;

use std::future::Future;

use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::superposition::{ConfigContext, GetValue};

use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    routes::SessionState,
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

/// Trait for configuration definitions
///
/// Each configuration type implements this trait to define how its value should be
/// retrieved from Superposition, database, or default value.
pub trait Config {
    /// The output type of this configuration
    type Output: Default + ConfigType + Clone;

    /// Get the Superposition key for this config
    const SUPERPOSITION_KEY: &'static str;

    /// Get the database key suffix for this config
    const KEY: &'static str;

    /// Get the default value for this config
    const DEFAULT_VALUE: Self::Output;

    /// Fetch the configuration value from Superposition with database fallback
    ///
    /// # Arguments
    /// * `state` - The session state containing storage and superposition client
    /// * `db_key` - The database key to use for fallback
    /// * `context` - Optional evaluation context for Superposition
    ///
    /// # Returns
    /// * `Self::Output` - The configuration value (never returns error, always returns default on failure)
    fn fetch(
        state: &SessionState,
        db_key: &str,
        context: Option<ConfigContext>,
    ) -> impl Future<Output = Self::Output>
    where
        Self: Sized,
        open_feature::Client: GetValue<<Self as Config>::Output>,
    {
        async move {
            let superposition_key = Self::SUPERPOSITION_KEY;
            let default_value = Self::DEFAULT_VALUE;

            // Try superposition first if available
            let superposition_result: Option<Self::Output> = if let Some(ref superposition_client) =
                state.superposition_service
            {
                match superposition_client
                    .get_config_value::<Self::Output>(superposition_key, context.as_ref())
                    .await
                {
                    Ok(value) => {
                        let result: Option<Self::Output> = Some(value);
                        result
                    }
                    Err(err) => {
                        router_env::logger::warn!(
                            "Failed to retrieve config from superposition, falling back to application default: {:?}",
                            err
                        );
                        Some(default_value.clone())
                    }
                }
            } else {
                None
            };

            // Use superposition result or fall back to database
            if let Some(value) = superposition_result {
                router_env::logger::info!(
                    "Successfully fetched config '{}' from superposition",
                    superposition_key
                );
                value
            } else {
                router_env::logger::info!("Retrieving config from database for key '{}'", db_key);
                let config_result = state
                    .store
                    .find_config_by_key_unwrap_or(
                        db_key,
                        Some(default_value.to_config_string().unwrap_or_default()),
                    )
                    .await;

                match config_result {
                    Ok(config) => {
                        Self::Output::from_config_str(&config.config).unwrap_or(default_value)
                    }
                    Err(_) => default_value,
                }
            }
        }
    }
}
