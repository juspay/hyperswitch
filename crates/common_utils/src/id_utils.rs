//! ID generation utils
use uuid::Uuid;

use crate::{errors, generate_id};

pub(crate) const ID_LENGTH: usize = 20;
pub(crate) const MAX_ID_LENGTH: usize = 64;

/// Get or generate an ID for a given key and prefix
pub fn get_or_generate_id(
    key: &str,
    provided_id: &Option<String>,
    prefix: &str,
) -> Result<String, errors::IdFormatError> {
    let validate_id = |id| validate_id(id, key);
    provided_id
        .clone()
        .map_or(Ok(generate_id(ID_LENGTH, prefix)), validate_id)
}

/// Get or generate a UUID for a given key
pub fn get_or_generate_uuid(
    key: &str,
    provided_id: Option<&String>,
) -> Result<String, errors::IdFormatError> {
    let validate_id = |id: String| validate_uuid(id, key);
    provided_id
        .cloned()
        .map_or(Ok(generate_uuid()), validate_id)
}

/// throw an error for invalid id format
fn invalid_id_format_error(key: &str) -> errors::IdFormatError {
    errors::IdFormatError::InvalidIDFormat {
        field_name: key.to_string(),
        expected_format: format!("length should be less than {} characters", MAX_ID_LENGTH),
    }
}

/// Validate an ID
pub fn validate_id(id: String, key: &str) -> Result<String, errors::IdFormatError> {
    if id.len() > MAX_ID_LENGTH {
        Err(invalid_id_format_error(key))
    } else {
        Ok(id)
    }
}

/// Validate a UUID
pub fn validate_uuid(uuid: String, key: &str) -> Result<String, errors::IdFormatError> {
    match (Uuid::parse_str(&uuid), uuid.len() > MAX_ID_LENGTH) {
        (Ok(_), false) => Ok(uuid),
        (_, _) => Err(invalid_id_format_error(key)),
    }
}

/// Generate an UUID
#[inline]
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}
