use diesel_models::observability::raw_json::RawJson;
use error_stack::ResultExt;
use serde_json::value::RawValue;

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
