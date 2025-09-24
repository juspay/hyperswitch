use common_utils::errors::CustomResult;
use error_stack::ResultExt;
#[cfg(feature = "superposition")]
use external_services::superposition::ConfigContext;

use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    routes::SessionState,
    services::ApplicationResponse,
    types::{api, transformers::ForeignInto},
};

/// Context for configuration retrieval - defines the strategy and parameters
#[derive(Debug, Clone)]
pub enum ConfigRetrieveContext {
    /// Retrieve configuration from database only
    DatabaseOnly {
        /// Database key to retrieve
        key: String,
    },
    /// Retrieve from Superposition with database fallback
    #[cfg(feature = "superposition")]
    SuperpositionWithDatabaseFallback {
        /// Superposition configuration key
        superposition_key: String,
        /// Context for Superposition evaluation
        superposition_context: Option<ConfigContext>,
        /// Database fallback key
        database_key: String,
    },
    /// Retrieve from Superposition only (no fallback)
    #[cfg(feature = "superposition")]
    SuperpositionOnly {
        /// Superposition configuration key
        key: String,
        /// Context for Superposition evaluation
        context: Option<ConfigContext>,
    },
}

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

/// Get a boolean configuration value based on the specified context
pub async fn get_config_bool(
    state: &SessionState,
    context: ConfigRetrieveContext,
    default_value: bool,
) -> CustomResult<bool, errors::StorageError> {
    match context {
        ConfigRetrieveContext::DatabaseOnly { key } => {
            let config = state
                .store
                .find_config_by_key_unwrap_or(&key, Some(default_value.to_string()))
                .await?;

            config
                .config
                .parse::<bool>()
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionWithDatabaseFallback {
            superposition_key,
            superposition_context,
            database_key,
        } => {
            if let Some(ref superposition_client) = state.superposition_service {
                match superposition_client
                    .get_bool_value(&superposition_key, superposition_context.as_ref())
                    .await
                {
                    Ok(value) => return Ok(value),
                    Err(err) => {
                        router_env::logger::warn!(
                            "Failed to retrieve config from superposition, falling back to database: {:?}",
                            err
                        );
                    }
                }
            }

            let config = state
                .store
                .find_config_by_key_unwrap_or(&database_key, Some(default_value.to_string()))
                .await?;

            config
                .config
                .parse::<bool>()
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionOnly { key, context } => {
            if let Some(ref superposition_client) = state.superposition_service {
                superposition_client
                    .get_bool_value(&key, context.as_ref())
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
            } else {
                Err(errors::StorageError::ValueNotFound(
                    "Superposition client not available".to_string(),
                )
                .into())
            }
        }
    }
}

/// Get a string configuration value based on the specified context
pub async fn get_config_string(
    state: &SessionState,
    context: ConfigRetrieveContext,
    default_value: String,
) -> CustomResult<String, errors::StorageError> {
    match context {
        ConfigRetrieveContext::DatabaseOnly { key } => {
            let config = state
                .store
                .find_config_by_key_unwrap_or(&key, Some(default_value))
                .await?;

            Ok(config.config)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionWithDatabaseFallback {
            superposition_key,
            superposition_context,
            database_key,
        } => {
            if let Some(ref superposition_client) = state.superposition_service {
                match superposition_client
                    .get_string_value(&superposition_key, superposition_context.as_ref())
                    .await
                {
                    Ok(value) => return Ok(value),
                    Err(err) => {
                        router_env::logger::warn!(
                            "Failed to retrieve config from superposition, falling back to database: {:?}",
                            err
                        );
                    }
                }
            }

            let config = state
                .store
                .find_config_by_key_unwrap_or(&database_key, Some(default_value))
                .await?;

            Ok(config.config)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionOnly { key, context } => {
            if let Some(ref superposition_client) = state.superposition_service {
                superposition_client
                    .get_string_value(&key, context.as_ref())
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
            } else {
                Err(errors::StorageError::ValueNotFound(
                    "Superposition client not available".to_string(),
                )
                .into())
            }
        }
    }
}

/// Get an integer configuration value based on the specified context
pub async fn get_config_int(
    state: &SessionState,
    context: ConfigRetrieveContext,
    default_value: i64,
) -> CustomResult<i64, errors::StorageError> {
    match context {
        ConfigRetrieveContext::DatabaseOnly { key } => {
            let config = state
                .store
                .find_config_by_key_unwrap_or(&key, Some(default_value.to_string()))
                .await?;

            config
                .config
                .parse::<i64>()
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionWithDatabaseFallback {
            superposition_key,
            superposition_context,
            database_key,
        } => {
            if let Some(ref superposition_client) = state.superposition_service {
                match superposition_client
                    .get_int_value(&superposition_key, superposition_context.as_ref())
                    .await
                {
                    Ok(value) => return Ok(value),
                    Err(err) => {
                        router_env::logger::warn!(
                            "Failed to retrieve config from superposition, falling back to database: {:?}",
                            err
                        );
                    }
                }
            }

            let config = state
                .store
                .find_config_by_key_unwrap_or(&database_key, Some(default_value.to_string()))
                .await?;

            config
                .config
                .parse::<i64>()
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionOnly { key, context } => {
            if let Some(ref superposition_client) = state.superposition_service {
                superposition_client
                    .get_int_value(&key, context.as_ref())
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
            } else {
                Err(errors::StorageError::ValueNotFound(
                    "Superposition client not available".to_string(),
                )
                .into())
            }
        }
    }
}

/// Get a float configuration value based on the specified context
pub async fn get_config_float(
    state: &SessionState,
    context: ConfigRetrieveContext,
    default_value: f64,
) -> CustomResult<f64, errors::StorageError> {
    match context {
        ConfigRetrieveContext::DatabaseOnly { key } => {
            let config = state
                .store
                .find_config_by_key_unwrap_or(&key, Some(default_value.to_string()))
                .await?;

            config
                .config
                .parse::<f64>()
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionWithDatabaseFallback {
            superposition_key,
            superposition_context,
            database_key,
        } => {
            if let Some(ref superposition_client) = state.superposition_service {
                match superposition_client
                    .get_float_value(&superposition_key, superposition_context.as_ref())
                    .await
                {
                    Ok(value) => return Ok(value),
                    Err(err) => {
                        router_env::logger::warn!(
                            "Failed to retrieve config from superposition, falling back to database: {:?}",
                            err
                        );
                    }
                }
            }

            let config = state
                .store
                .find_config_by_key_unwrap_or(&database_key, Some(default_value.to_string()))
                .await?;

            config
                .config
                .parse::<f64>()
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionOnly { key, context } => {
            if let Some(ref superposition_client) = state.superposition_service {
                superposition_client
                    .get_float_value(&key, context.as_ref())
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
            } else {
                Err(errors::StorageError::ValueNotFound(
                    "Superposition client not available".to_string(),
                )
                .into())
            }
        }
    }
}

/// Get an object configuration value based on the specified context
pub async fn get_config_object<T>(
    state: &SessionState,
    context: ConfigRetrieveContext,
    default_value: T,
) -> CustomResult<T, errors::StorageError>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    match context {
        ConfigRetrieveContext::DatabaseOnly { key } => {
            let config = state
                .store
                .find_config_by_key_unwrap_or(
                    &key,
                    Some(
                        serde_json::to_string(&default_value)
                            .change_context(errors::StorageError::SerializationFailed)?,
                    ),
                )
                .await?;

            serde_json::from_str::<T>(&config.config)
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionWithDatabaseFallback {
            superposition_key,
            superposition_context,
            database_key,
        } => {
            if let Some(ref superposition_client) = state.superposition_service {
                match superposition_client
                    .get_object_value(&superposition_key, superposition_context.as_ref())
                    .await
                {
                    Ok(json_value) => {
                        return serde_json::from_value::<T>(json_value)
                            .change_context(errors::StorageError::DeserializationFailed);
                    }
                    Err(err) => {
                        router_env::logger::warn!(
                            "Failed to retrieve config from superposition, falling back to database: {:?}",
                            err
                        );
                    }
                }
            }

            let config = state
                .store
                .find_config_by_key_unwrap_or(
                    &database_key,
                    Some(
                        serde_json::to_string(&default_value)
                            .change_context(errors::StorageError::SerializationFailed)?,
                    ),
                )
                .await?;

            serde_json::from_str::<T>(&config.config)
                .change_context(errors::StorageError::DeserializationFailed)
        }
        #[cfg(feature = "superposition")]
        ConfigRetrieveContext::SuperpositionOnly { key, context } => {
            if let Some(ref superposition_client) = state.superposition_service {
                superposition_client
                    .get_object_value(&key, context.as_ref())
                    .await
                    .change_context(errors::StorageError::DeserializationFailed)
                    .and_then(|json_value| {
                        serde_json::from_value::<T>(json_value)
                            .change_context(errors::StorageError::DeserializationFailed)
                    })
            } else {
                Err(errors::StorageError::ValueNotFound(
                    "Superposition client not available".to_string(),
                )
                .into())
            }
        }
    }
}
