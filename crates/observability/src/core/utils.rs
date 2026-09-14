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
                .attach_printable("Failed to build an empty JSON list")
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

pub fn within_width(
    value: &str,
    field_name: &'static str,
    max_chars: usize,
) -> ObservabilityApiResult<()> {
    let chars = value.chars().count();
    if chars > max_chars {
        Err(
            report!(ObservabilityError::InvalidDataValue { field_name }).attach_printable(format!(
                "The {field_name} is {chars} characters, over the {max_chars} the column holds"
            )),
        )?;
    }

    Ok(())
}

pub fn optional_within_width(
    value: Option<&str>,
    field_name: &'static str,
    max_chars: usize,
) -> ObservabilityApiResult<()> {
    value.map_or(Ok(()), |value| within_width(value, field_name, max_chars))
}

pub fn truncate_to_millisecond(value: PrimitiveDateTime) -> PrimitiveDateTime {
    value
        .replace_millisecond(value.millisecond())
        .unwrap_or(value)
}

pub enum TransactionError {
    Observability(error_stack::Report<ObservabilityError>),
    Database(diesel::result::Error),
}

impl TransactionError {
    pub fn into_report(self, message: &'static str) -> error_stack::Report<ObservabilityError> {
        match self {
            Self::Observability(error) => error,
            Self::Database(error) => report!(error)
                .change_context(ObservabilityError::InternalServerError)
                .attach_printable(message),
        }
    }
}

impl From<diesel::result::Error> for TransactionError {
    fn from(error: diesel::result::Error) -> Self {
        Self::Database(error)
    }
}

impl From<error_stack::Report<ObservabilityError>> for TransactionError {
    fn from(error: error_stack::Report<ObservabilityError>) -> Self {
        Self::Observability(error)
    }
}
