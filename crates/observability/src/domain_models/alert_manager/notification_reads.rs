use api_models::observability::alert_manager::notification_reads as api;
use common_utils::date_time;
use diesel_models::observability::alert_manager::notification_reads as storage;
use time::PrimitiveDateTime;

use crate::{
    domain_models::{optional_text, LONG_TEXT_MAX_CHARS},
    errors::{ObservabilityApiResult, ObservabilityError},
};

#[derive(Clone, Debug)]
pub struct NotificationReadsNew {
    pub user_name: String,
    pub last_read_at: PrimitiveDateTime,
}

#[derive(Clone, Debug)]
pub struct NotificationReads {
    pub user_name: String,
    pub last_read_at: PrimitiveDateTime,
}

impl NotificationReadsNew {
    fn validate(&self) -> ObservabilityApiResult<()> {
        optional_text("user_name", Some(&self.user_name), LONG_TEXT_MAX_CHARS)
    }
}

impl TryFrom<api::NotificationReadsUpsertRequest> for NotificationReadsNew {
    type Error = error_stack::Report<ObservabilityError>;

    fn try_from(request: api::NotificationReadsUpsertRequest) -> Result<Self, Self::Error> {
        let new = Self {
            user_name: request.user_name,
            last_read_at: date_time::now(),
        };

        new.validate()?;

        Ok(new)
    }
}

impl From<NotificationReadsNew> for storage::NotificationReadsNew {
    fn from(new: NotificationReadsNew) -> Self {
        Self {
            user_name: new.user_name,
            last_read_at: new.last_read_at,
        }
    }
}

impl From<storage::NotificationReads> for NotificationReads {
    fn from(row: storage::NotificationReads) -> Self {
        Self {
            user_name: row.user_name,
            last_read_at: row.last_read_at,
        }
    }
}

impl From<NotificationReads> for api::NotificationReadsResponse {
    fn from(read: NotificationReads) -> Self {
        Self {
            user_name: read.user_name,
            last_read_at: read.last_read_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use time::macros::datetime;

    use super::*;

    fn request(user_name: &str) -> api::NotificationReadsUpsertRequest {
        api::NotificationReadsUpsertRequest {
            user_name: user_name.to_owned(),
        }
    }

    #[test]
    fn an_upsert_stamps_the_current_time() {
        let before = date_time::now();
        let new = NotificationReadsNew::try_from(request("ops.engineer@example.com")).unwrap();
        let after = date_time::now();

        assert!(before <= new.last_read_at);
        assert!(new.last_read_at <= after);
    }

    #[test]
    fn an_upsert_keeps_the_user_name_exactly_as_sent() {
        let new = NotificationReadsNew::try_from(request("  Ops.Engineer@Example.com ")).unwrap();

        assert_eq!(new.user_name, "  Ops.Engineer@Example.com ");
    }

    #[test]
    fn an_empty_user_name_is_accepted() {
        let new = NotificationReadsNew::try_from(request("")).unwrap();

        assert_eq!(new.user_name, "");
    }

    #[test]
    fn a_user_name_of_255_multibyte_characters_is_accepted() {
        let user_name = "é".repeat(LONG_TEXT_MAX_CHARS);

        assert!(NotificationReadsNew::try_from(request(&user_name)).is_ok());
    }

    #[test]
    fn a_user_name_over_255_characters_is_refused() {
        let user_name = "a".repeat(LONG_TEXT_MAX_CHARS + 1);
        let error = NotificationReadsNew::try_from(request(&user_name)).unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    #[test]
    fn a_stored_row_reaches_the_response_unchanged() {
        let row = storage::NotificationReads {
            user_name: "ops.engineer@example.com".to_owned(),
            last_read_at: datetime!(2026-09-15 09:42:10.512),
        };

        let response = api::NotificationReadsResponse::from(NotificationReads::from(row));

        assert_eq!(response.user_name, "ops.engineer@example.com");
        assert_eq!(response.last_read_at, datetime!(2026-09-15 09:42:10.512));
    }

    #[test]
    fn the_response_renders_last_read_at_as_iso8601_with_milliseconds() {
        let response = api::NotificationReadsResponse {
            user_name: "ops.engineer@example.com".to_owned(),
            last_read_at: datetime!(2026-09-15 09:42:10.512),
        };

        let rendered = serde_json::to_value(response).unwrap();

        assert_eq!(rendered["last_read_at"], "2026-09-15T09:42:10.512Z");
    }
}
