use diesel_models::observability::raw_json::RawJson;
use error_stack::{report, ResultExt};
use serde_json::value::RawValue;
use time::PrimitiveDateTime;

use crate::errors::{ObservabilityApiResult, ObservabilityError};

pub const NAME_MAX_CHARS: usize = 64;

pub const VALUE_MAX_CHARS: usize = 255;

const EMPTY_LIST: &str = "[]";

const EMPTY_JSON_TEXT: &str = "{}";

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

pub fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

pub fn empty_json_text() -> serde_json::Value {
    serde_json::Value::String(EMPTY_JSON_TEXT.to_owned())
}

pub fn within_width(value: &str, field: &str, max_chars: usize) -> ObservabilityApiResult<()> {
    let chars = value.chars().count();
    if chars > max_chars {
        Err(report!(ObservabilityError::InvalidDataValue {
            field_name: field.to_owned(),
        })
        .attach_printable(format!(
            "The {field} is {chars} characters, over the {max_chars} the column holds"
        )))?;
    }

    Ok(())
}

pub fn optional_within_width(
    value: Option<&str>,
    field: &str,
    max_chars: usize,
) -> ObservabilityApiResult<()> {
    value.map_or(Ok(()), |value| within_width(value, field, max_chars))
}

pub fn truncate_to_millisecond(value: PrimitiveDateTime) -> PrimitiveDateTime {
    value
        .replace_millisecond(value.millisecond())
        .unwrap_or(value)
}
