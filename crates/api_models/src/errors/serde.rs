use serde::{ser::SerializeMap, Serialize};

use super::types::ApiErrorResponse;

impl Serialize for ApiErrorResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("error_type", self.error_type())?;
        map.serialize_entry(
            "error_code",
            &format!(
                "{}_{}",
                self.get_internal_error().sub_code,
                self.get_internal_error().error_identifier
            ),
        )?;
        map.serialize_entry("error_message", &self.get_internal_error().error_message)?;
        map.end()
    }
}
