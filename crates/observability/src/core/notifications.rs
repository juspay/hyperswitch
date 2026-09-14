use diesel_models::observability::notification_reads::{NotificationRead, NotificationReadNew};
use error_stack::ResultExt;

use crate::{
    auth::UserName,
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
    types::notifications::NotificationWatermarkResponse,
};

pub async fn retrieve_notification_watermark(
    state: AppState,
    user_name: UserName,
) -> ObservabilityApiResult<NotificationWatermarkResponse> {
    let connection = state.database_connection().await?;

    NotificationRead::find_by_user_name(&connection, user_name.get_secret())
        .await
        .to_not_found_response(ObservabilityError::NotificationWatermarkNotFound)
        .attach_printable("Failed to find a notification watermark")
        .map(NotificationWatermarkResponse::from)
}

pub async fn upsert_notification_watermark(
    state: AppState,
    user_name: UserName,
) -> ObservabilityApiResult<NotificationWatermarkResponse> {
    let connection = state.database_connection().await?;

    NotificationReadNew {
        user_name: user_name.get_secret(),
        last_read_at: common_utils::date_time::now(),
    }
    .upsert(&connection)
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to save a notification watermark")
    .map(NotificationWatermarkResponse::from)
}
