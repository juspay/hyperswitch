use serde::Serialize;
use time::PrimitiveDateTime;

use super::ReadStatus;

#[derive(Debug, Serialize)]
pub struct WatermarkResponse {
    pub status: ReadStatus,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub last_read_at: Option<PrimitiveDateTime>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn body_of<T: Serialize>(value: &T) -> serde_json::Value {
        serde_json::to_value(value).unwrap()
    }

    #[test]
    fn a_watermark_that_was_never_set_is_an_answer_and_not_an_error() {
        let body = body_of(&WatermarkResponse {
            status: ReadStatus::Absent,
            last_read_at: None,
        });

        assert_eq!(body["status"], "absent");
        assert!(body["last_read_at"].is_null());
    }
}
