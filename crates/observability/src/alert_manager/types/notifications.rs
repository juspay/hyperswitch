//! The wire contract for the notification bell's read watermark.
//!
//! One resource, addressed by the user in [`X_USER_NAME`](super::X_USER_NAME) rather than in the
//! path. A user who has never cleared their feed has no row, which is an answer and not an error.

use serde::Serialize;
use time::PrimitiveDateTime;

use super::ReadStatus;

/// What the notification watermark routes return.
#[derive(Debug, Serialize)]
pub struct WatermarkResponse {
    /// Whether a watermark exists. Always present: a user who has never cleared their feed has no
    /// row, and the bell must read that as "everything is unread" rather than as a failed read.
    pub status: ReadStatus,
    /// The instant the feed was last cleared, or `null` when it never was.
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

    /// A user who has never cleared the feed has no row, and the bell reads that as "everything is
    /// unread" — which it can only do if the response says so rather than failing.
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
