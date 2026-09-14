use serde::Serialize;
use time::PrimitiveDateTime;

use super::ReadStatus;

#[derive(Debug, Serialize)]
pub struct WatermarkResponse {
    pub status: ReadStatus,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_read_at: Option<PrimitiveDateTime>,
}
