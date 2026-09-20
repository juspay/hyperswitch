use error_stack::{report, ResultExt};

use crate::errors::{ObservabilityApiResult, ObservabilityError};

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
        assert!(required_text("name", "   ", 64).is_err());
        assert!(required_text("name", "sr_drop", 64).is_ok());
    }

    #[test]
    fn the_limit_counts_characters_not_bytes() {
        let at_limit = "é".repeat(64);
        let over = "é".repeat(65);

        assert!(optional_text("author", Some(&at_limit), 64).is_ok());
        assert!(optional_text("author", Some(&over), 64).is_err());
        assert!(optional_text("author", None, 64).is_ok());
    }

    #[test]
    fn a_negative_count_is_refused() {
        assert!(non_negative("period", Some(-1)).is_err());
        assert!(non_negative("period", Some(0)).is_ok());
        assert!(non_negative("period", None).is_ok());
    }
}
