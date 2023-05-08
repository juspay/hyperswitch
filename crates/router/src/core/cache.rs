use common_utils::errors::CustomResult;

use crate::{
    cache::CONFIG_CACHE,
    db::StorageInterface,
    services::{self},
};

pub async fn invalidate(
    store: &dyn StorageInterface,
    key: &str,
) -> CustomResult<
    services::api::ApplicationResponse<serde_json::Value>,
    crate::errors::ApiErrorResponse,
> {
    CONFIG_CACHE.remove(key).await;
    Ok(services::api::ApplicationResponse::StatusOk)
}
