use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};
use storage_impl::redis::cache::{redact_from_redis_and_publish, CacheKind};

use super::errors;
use crate::{routes::SessionState, services};

pub async fn invalidate(
    state: SessionState,
    key: &str,
) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ApiErrorResponse> {
    let store = state.store.as_ref();
    let result = redact_from_redis_and_publish(
        store.get_cache_store().as_ref(),
        [CacheKind::All(key.into())],
    )
    .await
    .change_context(errors::ApiErrorResponse::InternalServerError)?;

    // If the message was published to atleast one channel
    // then return status Ok
    if result > 0 {
        Ok(services::api::ApplicationResponse::StatusOk)
    } else {
        Err(report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Failed to invalidate cache"))
    }
}
