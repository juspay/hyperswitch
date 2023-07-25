use error_stack::ResultExt;

use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    db::StorageInterface,
    services::ApplicationResponse,
    types::{api, transformers::ForeignInto},
};

pub async fn set_config(
    store: &dyn StorageInterface,
    config: api::Config,
) -> RouterResponse<api::Config> {
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

pub async fn read_config(store: &dyn StorageInterface, key: &str) -> RouterResponse<api::Config> {
    let config = store
        .find_config_by_key_cached(key)
        .await
        .to_not_found_response(errors::ApiErrorResponse::ConfigNotFound)?;
    Ok(ApplicationResponse::Json(config.foreign_into()))
}

pub async fn update_config(
    store: &dyn StorageInterface,
    config_update: &api::ConfigUpdate,
) -> RouterResponse<api::Config> {
    let config = store
        .update_config_cached(&config_update.key, config_update.foreign_into())
        .await
        .to_not_found_response(errors::ApiErrorResponse::ConfigNotFound)?;
    Ok(ApplicationResponse::Json(config.foreign_into()))
}
