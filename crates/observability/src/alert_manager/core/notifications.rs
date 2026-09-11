//! Per-request logic for the notification bell's read watermark.

use diesel_models::observability::notification_reads::NotificationRead;
use error_stack::{report, ResultExt};

use crate::{
    alert_manager::types::{notifications::WatermarkResponse, ReadStatus, UserName},
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

/// `notification_reads.user_name` is `VARCHAR(255)`.
const USER_NAME_MAX_BYTES: usize = 255;

/// One user's watermark, or the fact that they have never cleared their feed.
pub async fn read(state: AppState, user: UserName) -> ObservabilityApiResult<WatermarkResponse> {
    let user_name = validated(&user)?;
    let connection = state.database_connection().await?;

    let watermark = NotificationRead::find_by_user_name(&connection, user_name)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to read a notification watermark")?;

    Ok(match watermark {
        Some(watermark) => WatermarkResponse {
            status: ReadStatus::Found,
            last_read_at: Some(watermark.last_read_at),
        },
        // Not an error and not a `404`.
        None => WatermarkResponse {
            status: ReadStatus::Absent,
            last_read_at: None,
        },
    })
}

/// Move the user's watermark to now.
pub async fn mark_read(
    state: AppState,
    user: UserName,
) -> ObservabilityApiResult<WatermarkResponse> {
    let user_name = validated(&user)?;
    let connection = state.database_connection().await?;

    let watermark = NotificationRead {
        user_name: user_name.to_owned(),
        last_read_at: common_utils::date_time::now(),
    }
    .upsert(&connection)
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to save a notification watermark")?;

    // `found` rather than a write status of its own:
    Ok(WatermarkResponse {
        status: ReadStatus::Found,
        last_read_at: Some(watermark.last_read_at),
    })
}

/// Check the asserted name against its column's width.
fn validated(user: &UserName) -> ObservabilityApiResult<&str> {
    let user_name = user.as_str();

    if user_name.len() > USER_NAME_MAX_BYTES {
        Err(
            report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                "The user name is {} bytes, over the {USER_NAME_MAX_BYTES} the column holds",
                user_name.len()
            )),
        )?;
    }

    Ok(user_name)
}
