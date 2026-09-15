//! Per-request logic for alert definitions.

use api_models::observability::alerts_info::{AlertsInfoCreateRequest, AlertsInfoResponse};
use error_stack::{report, ResultExt};

use crate::{
    domain::alerts_info::AlertsInfoNew,
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

/// Longest value a `VARCHAR(64)` column holds.
const SHORT_TEXT_MAX_CHARS: usize = 64;
/// Longest value a `VARCHAR(255)` column holds.
const LONG_TEXT_MAX_CHARS: usize = 255;

/// Store a new alert definition.
pub async fn create_alert_info(
    state: AppState,
    request: AlertsInfoCreateRequest,
) -> ObservabilityApiResult<AlertsInfoResponse> {
    let new = AlertsInfoNew::from(request);
    validate(&new)?;

    let stored = state
        .store
        .insert_alert_info(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to insert into alerts_info")?;

    Ok(AlertsInfoResponse::from(stored))
}

/// Reject what the table would refuse, so a bad value is a `400` rather than a database error.
fn validate(new: &AlertsInfoNew) -> ObservabilityApiResult<()> {
    required_text("name", &new.name)?;
    required_text("product", &new.product)?;

    optional_text("dimensions", new.dimensions.as_deref(), LONG_TEXT_MAX_CHARS)?;
    optional_text(
        "default_channel",
        new.default_channel.as_deref(),
        SHORT_TEXT_MAX_CHARS,
    )?;
    optional_text("author", new.author.as_deref(), SHORT_TEXT_MAX_CHARS)?;
    optional_text("approver", new.approver.as_deref(), SHORT_TEXT_MAX_CHARS)?;

    non_negative("period", new.period)?;
    non_negative("history_window", new.history_window)?;
    non_negative("call_period", new.call_period)?;

    Ok(())
}

fn required_text(field: &str, value: &str) -> ObservabilityApiResult<()> {
    if value.trim().is_empty() {
        Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable(format!("{field} must not be empty"))?
    }
    optional_text(field, Some(value), SHORT_TEXT_MAX_CHARS)
}

fn optional_text(field: &str, value: Option<&str>, max_chars: usize) -> ObservabilityApiResult<()> {
    if value.is_some_and(|value| value.chars().count() > max_chars) {
        Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable(format!("{field} must be at most {max_chars} characters"))?
    }
    Ok(())
}

fn non_negative(field: &str, value: Option<i32>) -> ObservabilityApiResult<()> {
    if value.is_some_and(|value| value < 0) {
        Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable(format!("{field} must not be negative"))?
    }
    Ok(())
}
