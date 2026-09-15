use api_models::observability::notification_reads::{
    NotificationReadsResponse, NotificationReadsRetrieveRequest, NotificationReadsUpsertRequest,
};
use error_stack::ResultExt;

use crate::{
    domain_models::notification_reads::NotificationReadsNew,
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
};

pub async fn retrieve_notification_read(
    state: AppState,
    request: NotificationReadsRetrieveRequest,
) -> ObservabilityApiResult<NotificationReadsResponse> {
    let stored = state
        .store
        .find_notification_read_by_user_name(&request.user_name)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)
        .attach_printable("Failed to find in notification_reads")?;

    Ok(NotificationReadsResponse::from(stored))
}

pub async fn upsert_notification_read(
    state: AppState,
    request: NotificationReadsUpsertRequest,
) -> ObservabilityApiResult<NotificationReadsResponse> {
    let new = NotificationReadsNew::try_from(request)?;

    let stored = state
        .store
        .upsert_notification_read(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to upsert into notification_reads")?;

    Ok(NotificationReadsResponse::from(stored))
}
