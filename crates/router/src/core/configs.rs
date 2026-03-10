use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use external_services::superposition::ConfigContext;

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

/// Get a boolean configuration value with superposition and database fallback
pub async fn get_config_bool(
    state: &SessionState,
    superposition_key: &str,
    db_key: &str,
    context: Option<ConfigContext>,
    default_value: bool,
) -> CustomResult<bool, errors::StorageError> {
    // Try superposition first if available
    let superposition_result = if let Some(ref superposition_client) = state.superposition_service {
        match superposition_client
            .get_bool_value(superposition_key, context.as_ref())
            .await
        {
            Ok(value) => Some(value),
            Err(err) => {
                router_env::logger::warn!(
                    "Failed to retrieve config from superposition, falling back to database: {:?}",
                    err
                );
                None
            }
        }
    } else {
        None
    };

    // Use superposition result or fall back to database
    if let Some(value) = superposition_result {
        Ok(value)
    } else {
        let config = state
            .store
            .find_config_by_key_unwrap_or(db_key, Some(default_value.to_string()))
            .await?;

        config
            .config
            .parse::<bool>()
            .change_context(errors::StorageError::DeserializationFailed)
    }
}

/// Get a string configuration value with superposition and database fallback
pub async fn get_config_string(
    state: &SessionState,
    superposition_key: &str,
    db_key: &str,
    context: Option<ConfigContext>,
    default_value: String,
) -> CustomResult<String, errors::StorageError> {
    // Try superposition first if available
    let superposition_result = if let Some(ref superposition_client) = state.superposition_service {
        match superposition_client
            .get_string_value(superposition_key, context.as_ref())
            .await
        {
            Ok(value) => Some(value),
            Err(err) => {
                router_env::logger::warn!(
                    "Failed to retrieve config from superposition, falling back to database: {:?}",
                    err
                );
                None
            }
        }
    } else {
        None
    };

    // Use superposition result or fall back to database
    if let Some(value) = superposition_result {
        Ok(value)
    } else {
        let config = state
            .store
            .find_config_by_key_unwrap_or(db_key, Some(default_value))
            .await?;

        Ok(config.config)
    }
}

/// Get an integer configuration value with superposition and database fallback
pub async fn get_config_int(
    state: &SessionState,
    superposition_key: &str,
    db_key: &str,
    context: Option<ConfigContext>,
    default_value: i64,
) -> CustomResult<i64, errors::StorageError> {
    // Try superposition first if available
    let superposition_result = if let Some(ref superposition_client) = state.superposition_service {
        match superposition_client
            .get_int_value(superposition_key, context.as_ref())
            .await
        {
            Ok(value) => Some(value),
            Err(err) => {
                router_env::logger::warn!(
                    "Failed to retrieve config from superposition, falling back to database: {:?}",
                    err
                );
                None
            }
        }
    } else {
        None
    };

    // Use superposition result or fall back to database
    if let Some(value) = superposition_result {
        Ok(value)
    } else {
        let config = state
            .store
            .find_config_by_key_unwrap_or(db_key, Some(default_value.to_string()))
            .await?;

        config
            .config
            .parse::<i64>()
            .change_context(errors::StorageError::DeserializationFailed)
    }
}

/// Get a float configuration value with superposition and database fallback
pub async fn get_config_float(
    state: &SessionState,
    superposition_key: &str,
    db_key: &str,
    context: Option<ConfigContext>,
    default_value: f64,
) -> CustomResult<f64, errors::StorageError> {
    // Try superposition first if available
    let superposition_result = if let Some(ref superposition_client) = state.superposition_service {
        match superposition_client
            .get_float_value(superposition_key, context.as_ref())
            .await
        {
            Ok(value) => Some(value),
            Err(err) => {
                router_env::logger::warn!(
                    "Failed to retrieve config from superposition, falling back to database: {:?}",
                    err
                );
                None
            }
        }
    } else {
        None
    };

    // Use superposition result or fall back to database
    if let Some(value) = superposition_result {
        Ok(value)
    } else {
        let config = state
            .store
            .find_config_by_key_unwrap_or(db_key, Some(default_value.to_string()))
            .await?;

        config
            .config
            .parse::<f64>()
            .change_context(errors::StorageError::DeserializationFailed)
    }
}

/// Get an object configuration value with superposition and database fallback
pub async fn get_config_object<T>(
    state: &SessionState,
    superposition_key: &str,
    db_key: &str,
    context: Option<ConfigContext>,
    default_value: T,
) -> CustomResult<T, errors::StorageError>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    // Try superposition first if available
    let superposition_result = if let Some(ref superposition_client) = state.superposition_service {
        match superposition_client
            .get_object_value(superposition_key, context.as_ref())
            .await
        {
            Ok(json_value) => Some(
                serde_json::from_value::<T>(json_value)
                    .change_context(errors::StorageError::DeserializationFailed),
            ),
            Err(err) => {
                router_env::logger::warn!(
                    "Failed to retrieve config from superposition, falling back to database: {:?}",
                    err
                );
                None
            }
        }
    } else {
        None
    };

    // Use superposition result or fall back to database
    if let Some(superposition_result) = superposition_result {
        superposition_result
    } else {
        let config = state
            .store
            .find_config_by_key_unwrap_or(
                db_key,
                Some(
                    serde_json::to_string(&default_value)
                        .change_context(errors::StorageError::SerializationFailed)?,
                ),
            )
            .await?;

        serde_json::from_str::<T>(&config.config)
            .change_context(errors::StorageError::DeserializationFailed)
    }
}
