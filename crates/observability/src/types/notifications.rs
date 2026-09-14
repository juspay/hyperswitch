use diesel_models::observability::notification_reads::NotificationRead;
use serde::Serialize;
use time::PrimitiveDateTime;

#[derive(Debug, Serialize)]
pub struct NotificationWatermarkResponse {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_read_at: PrimitiveDateTime,
}

impl From<NotificationRead> for NotificationWatermarkResponse {
    fn from(watermark: NotificationRead) -> Self {
        Self {
            last_read_at: watermark.last_read_at,
        }
    }
}
