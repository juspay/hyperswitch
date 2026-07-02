use error_stack::{Report, ResultExt};
use serde_json::Value;

use crate::core::errors::ApiErrorResponse;
pub fn serialize_request_to_json<T: serde::Serialize>(
    request: &T,
) -> Result<Value, Report<ApiErrorResponse>> {
    serde_json::to_value(request)
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to serialize request payload to JSON")
}
