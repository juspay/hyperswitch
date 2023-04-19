use crate::{
    core::errors::{self, utils::StorageErrorExt, RouterResponse},
    db::StorageInterface,
    services::ApplicationResponse,
    types::{api, transformers::ForeignInto},
};

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
