use diesel_models::observability::raw_json::RawJson;
use error_stack::{report, ResultExt};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

const EMPTY_LIST: &str = "[]";

pub fn or_empty_list(column: Option<RawJson>) -> ObservabilityApiResult<RawJson> {
    column.map_or_else(
        || {
            RawValue::from_string(EMPTY_LIST.to_owned())
                .map(RawJson::from)
                .change_context(ObservabilityError::InternalServerError)
        },
        Ok,
    )
}

pub fn within_width(value: &str, field: &str, max_chars: usize) -> ObservabilityApiResult<()> {
    let chars = value.chars().count();
    if chars > max_chars {
        Err(
            report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                "The {field} is {chars} characters, over the {max_chars} the column holds"
            )),
        )?;
    }

    Ok(())
}

pub fn truncate_to_millisecond(value: PrimitiveDateTime) -> PrimitiveDateTime {
    value
        .replace_millisecond(value.millisecond())
        .unwrap_or(value)
}
