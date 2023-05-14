use common_utils::errors::CustomResult;

use super::errors;
use crate::{cache::CONFIG_CACHE, db::StorageInterface, services};

pub async fn invalidate(
    store: &dyn StorageInterface,
    key: &str,
) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ApiErrorResponse> {
    CONFIG_CACHE.remove(key).await;

    match store.get_redis_conn().delete_key(key).await {
        Ok(_) => Ok(services::api::ApplicationResponse::StatusOk),
        _ => Err(errors::ApiErrorResponse::InvalidRequestUrl.into()),
    }
}
