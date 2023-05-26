use common_utils::errors::CustomResult;

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
        Ok(_del_reply) => Ok(services::api::ApplicationResponse::StatusOk),
        _ => Err(errors::ApiErrorResponse::InvalidRequestUrl.into()),
    }
}
