//! The types the layers exchange, and the conversions between them.
//!
//! A model here sits between the API types in `api_models` and the rows in `diesel_models`, so
//! neither [`crate::core`] nor [`crate::db`] has to know the other's shape. Validation of what a
//! model may hold lives with the model, so a value that reaches [`crate::db`] is already one the
//! table will accept.
//!
//! Distinct from [`crate::domain`], which holds the traits that say what delivering an alert *is*.

pub mod alerts_dicts;
pub mod alerts_info;
pub mod notification_reads;

use error_stack::{report, ResultExt};

use crate::errors::{ObservabilityApiResult, ObservabilityError};

pub(crate) const SHORT_TEXT_MAX_CHARS: usize = 64;
pub(crate) const LONG_TEXT_MAX_CHARS: usize = 255;

pub(crate) fn required_text(
    field: &str,
    value: &str,
    max_chars: usize,
) -> ObservabilityApiResult<()> {
    if value.trim().is_empty() {
        Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable(format!("{field} must not be empty"))?
    }
    optional_text(field, Some(value), max_chars)
}

pub(crate) fn optional_text(
    field: &str,
    value: Option<&str>,
    max_chars: usize,
) -> ObservabilityApiResult<()> {
    if value.is_some_and(|value| value.chars().count() > max_chars) {
        Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable(format!("{field} must be at most {max_chars} characters"))?
    }
    Ok(())
}

pub(crate) fn non_negative(field: &str, value: Option<i32>) -> ObservabilityApiResult<()> {
    if value.is_some_and(|value| value < 0) {
        Err(report!(ObservabilityError::InvalidRequest))
            .attach_printable(format!("{field} must not be negative"))?
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_required_value_is_refused() {
        assert!(required_text("name", "   ", SHORT_TEXT_MAX_CHARS).is_err());
        assert!(required_text("name", "sr_drop", SHORT_TEXT_MAX_CHARS).is_ok());
    }

    #[test]
    fn the_limit_counts_characters_not_bytes() {
        let at_limit = "é".repeat(SHORT_TEXT_MAX_CHARS);
        let over = "é".repeat(SHORT_TEXT_MAX_CHARS + 1);

        assert!(optional_text("author", Some(&at_limit), SHORT_TEXT_MAX_CHARS).is_ok());
        assert!(optional_text("author", Some(&over), SHORT_TEXT_MAX_CHARS).is_err());
        assert!(optional_text("author", None, SHORT_TEXT_MAX_CHARS).is_ok());
    }

    #[test]
    fn a_negative_count_is_refused() {
        assert!(non_negative("period", Some(-1)).is_err());
        assert!(non_negative("period", Some(0)).is_ok());
        assert!(non_negative("period", None).is_ok());
    }
}
