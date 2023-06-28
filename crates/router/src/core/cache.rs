use common_utils::errors::CustomResult;
use error_stack::{report, ResultExt};

use super::errors;
use crate::{
    cache::CacheKind,
    db::{cache::publish_into_redact_channel, StorageInterface},
    services,
};

pub async fn invalidate(
    store: &dyn StorageInterface,
    key: &str,
) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ApiErrorResponse> {
    let result = publish_into_redact_channel(store, CacheKind::All(key.into()))
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
