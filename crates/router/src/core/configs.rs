use common_utils::errors::CustomResult;
use error_stack::ResultExt;

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

/// Get a boolean configuration value with Superposition -> Database fallback
pub async fn get_config_bool(
    state: &SessionState,
    #[cfg(feature = "superposition")] superposition_key: &str,
    #[cfg(not(feature = "superposition"))] _superposition_key: &str,
    db_key: &str,
    #[cfg(feature = "superposition")] context: Option<std::collections::HashMap<String, String>>,
    #[cfg(not(feature = "superposition"))] _context: Option<
        std::collections::HashMap<String, String>,
    >,
    default_value: bool,
) -> CustomResult<bool, errors::StorageError> {
    #[cfg(feature = "superposition")]
    if let Some(ref superposition_client) = state.superposition_service {
        if let Ok(value) = superposition_client
            .get_bool_value(superposition_key, context.as_ref())
            .await
        {
            return Ok(value);
        }
    }

    let config = state
        .store
        .find_config_by_key_unwrap_or(db_key, Some(default_value.to_string()))
        .await?;

    config
        .config
        .parse::<bool>()
        .change_context(errors::StorageError::DeserializationFailed)
}

/// Get a string configuration value with Superposition -> Database fallback
pub async fn get_config_string(
    state: &SessionState,
    #[cfg(feature = "superposition")] superposition_key: &str,
    #[cfg(not(feature = "superposition"))] _superposition_key: &str,
    db_key: &str,
    #[cfg(feature = "superposition")] context: Option<std::collections::HashMap<String, String>>,
    #[cfg(not(feature = "superposition"))] _context: Option<
        std::collections::HashMap<String, String>,
    >,
    default_value: String,
) -> CustomResult<String, errors::StorageError> {
    #[cfg(feature = "superposition")]
    if let Some(ref superposition_client) = state.superposition_service {
        if let Ok(value) = superposition_client
            .get_string_value(superposition_key, context.as_ref())
            .await
        {
            return Ok(value);
        }
    }

    let config = state
        .store
        .find_config_by_key_unwrap_or(db_key, Some(default_value))
        .await?;

    Ok(config.config)
}

/// Get an integer configuration value with Superposition -> Database fallback
pub async fn get_config_int(
    state: &SessionState,
    #[cfg(feature = "superposition")] superposition_key: &str,
    #[cfg(not(feature = "superposition"))] _superposition_key: &str,
    db_key: &str,
    #[cfg(feature = "superposition")] context: Option<std::collections::HashMap<String, String>>,
    #[cfg(not(feature = "superposition"))] _context: Option<
        std::collections::HashMap<String, String>,
    >,
    default_value: i64,
) -> CustomResult<i64, errors::StorageError> {
    #[cfg(feature = "superposition")]
    if let Some(ref superposition_client) = state.superposition_service {
        if let Ok(value) = superposition_client
            .get_int_value(superposition_key, context.as_ref())
            .await
        {
            return Ok(value);
        }
    }

    let config = state
        .store
        .find_config_by_key_unwrap_or(db_key, Some(default_value.to_string()))
        .await?;

    config
        .config
        .parse::<i64>()
        .change_context(errors::StorageError::DeserializationFailed)
}

/// Get a float configuration value with Superposition -> Database fallback
pub async fn get_config_float(
    state: &SessionState,
    #[cfg(feature = "superposition")] superposition_key: &str,
    #[cfg(not(feature = "superposition"))] _superposition_key: &str,
    db_key: &str,
    #[cfg(feature = "superposition")] context: Option<std::collections::HashMap<String, String>>,
    #[cfg(not(feature = "superposition"))] _context: Option<
        std::collections::HashMap<String, String>,
    >,
    default_value: f64,
) -> CustomResult<f64, errors::StorageError> {
    #[cfg(feature = "superposition")]
    if let Some(ref superposition_client) = state.superposition_service {
        if let Ok(value) = superposition_client
            .get_float_value(superposition_key, context.as_ref())
            .await
        {
            return Ok(value);
        }
    }

    let config = state
        .store
        .find_config_by_key_unwrap_or(db_key, Some(default_value.to_string()))
        .await?;

    config
        .config
        .parse::<f64>()
        .change_context(errors::StorageError::DeserializationFailed)
}

/// Get an object configuration value with Superposition -> Database fallback
pub async fn get_config_object(
    state: &SessionState,
    #[cfg(feature = "superposition")] superposition_key: &str,
    #[cfg(not(feature = "superposition"))] _superposition_key: &str,
    db_key: &str,
    #[cfg(feature = "superposition")] context: Option<std::collections::HashMap<String, String>>,
    #[cfg(not(feature = "superposition"))] _context: Option<
        std::collections::HashMap<String, String>,
    >,
    default_value: serde_json::Value,
) -> CustomResult<serde_json::Value, errors::StorageError> {
    #[cfg(feature = "superposition")]
    if let Some(ref superposition_client) = state.superposition_service {
        if let Ok(struct_value) = superposition_client
            .get_object_value(superposition_key, context.as_ref())
            .await
        {
            // Convert StructValue to serde_json::Value
            let json_value = serde_json::to_value(struct_value)
                .change_context(errors::StorageError::DeserializationFailed)?;
            return Ok(json_value);
        }
    }

    let config = state
        .store
        .find_config_by_key_unwrap_or(db_key, Some(default_value.to_string()))
        .await?;

    serde_json::from_str::<serde_json::Value>(&config.config)
        .change_context(errors::StorageError::DeserializationFailed)
}
