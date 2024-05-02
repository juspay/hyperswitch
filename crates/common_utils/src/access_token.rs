//! Commonly used utilities for access token

use std::fmt::Display;

/// Create a key for fetching the access token from redis
pub fn create_access_token_key(
    merchant_id: impl Display,
    merchant_connector_id_or_connector_name: impl Display,
) -> String {
    format!("access_token_{merchant_id}_{merchant_connector_id_or_connector_name}")
}
