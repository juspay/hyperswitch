use common_utils::errors::CustomResult;
use redis_interface::DelReply;

use super::errors;
use crate::{
    cache::{ACCOUNTS_CACHE, CONFIG_CACHE},
    db::StorageInterface,
    services,
};

pub async fn invalidate(
    store: &dyn StorageInterface,
    key: &str,
) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ApiErrorResponse> {
    CONFIG_CACHE.remove(key).await;
    ACCOUNTS_CACHE.remove(key).await;

    match store.get_redis_conn().delete_key(key).await {
        Ok(DelReply::KeyDeleted) => Ok(services::api::ApplicationResponse::StatusOk),
        Ok(DelReply::KeyNotDeleted) => Err(errors::ApiErrorResponse::InvalidRequestUrl.into()),
        Err(error) => Err(error
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to invalidate cache")),
    }
}
