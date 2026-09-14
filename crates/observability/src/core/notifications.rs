use diesel_models::observability::notification_reads::{NotificationRead, NotificationReadNew};
use error_stack::ResultExt;

use crate::{
    auth::UserName,
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
    types::{notifications::WatermarkResponse, ReadStatus},
};

pub async fn read_watermark(
    state: AppState,
    user_name: UserName,
) -> ObservabilityApiResult<WatermarkResponse> {
    let connection = state.database_connection().await?;

    let watermark = NotificationRead::find_by_user_name(&connection, user_name.get_secret())
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to read a notification watermark")?;

    Ok(match watermark {
        Some(watermark) => WatermarkResponse {
            status: ReadStatus::Found,
            last_read_at: Some(watermark.last_read_at),
        },
        None => WatermarkResponse {
            status: ReadStatus::Absent,
            last_read_at: None,
        },
    })
}

pub async fn mark_read(
    state: AppState,
    user_name: UserName,
) -> ObservabilityApiResult<WatermarkResponse> {
    let connection = state.database_connection().await?;

    let watermark = NotificationReadNew {
        user_name: user_name.get_secret(),
        last_read_at: common_utils::date_time::now(),
    }
    .upsert(&connection)
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to save a notification watermark")?;

    Ok(WatermarkResponse {
        status: ReadStatus::Found,
        last_read_at: Some(watermark.last_read_at),
    })
}
