use common_utils::errors::CustomResult;

use crate::{
    core::errors,
    db::StorageInterface,
    services::{self},
};

pub async fn invalidate(
    store: &dyn StorageInterface,
    key: &str,
) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, crate::errors::ApiErrorResponse> {
    match store.invalidate(key).await {
        Ok(()) => Ok(services::api::ApplicationResponse::StatusOk),
        _ => Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidateCache
        )),
    }
}
