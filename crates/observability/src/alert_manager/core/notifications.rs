use diesel_models::observability::notification_reads::NotificationRead;
use error_stack::{report, ResultExt};

use crate::{
    alert_manager::types::{notifications::WatermarkResponse, ReadStatus, UserName},
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

const USER_NAME_MAX_BYTES: usize = 255;

pub async fn read_watermark(
    state: AppState,
    user: UserName,
) -> ObservabilityApiResult<WatermarkResponse> {
    let user_name = within_width(&user)?;
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
        None => WatermarkResponse {
            status: ReadStatus::Absent,
            last_read_at: None,
        },
    })
}

pub async fn mark_read(
    state: AppState,
    user: UserName,
) -> ObservabilityApiResult<WatermarkResponse> {
    let user_name = within_width(&user)?;
    let connection = state.database_connection().await?;

    let watermark = NotificationRead {
        user_name: user_name.to_owned(),
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

fn within_width(user: &UserName) -> ObservabilityApiResult<&str> {
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
