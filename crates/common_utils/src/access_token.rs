//! Commonly used utilities for access token
use std::fmt::Display;

use crate::id_type;

/// Create a default key for fetching the access token from redis
pub fn get_default_access_token_key(
    merchant_id: &id_type::MerchantId,
    merchant_connector_id_or_connector_name: impl Display,
) -> String {
    format!(
        "access_token_{}_{}",
        merchant_id.get_string_repr(),
        merchant_connector_id_or_connector_name
    )
}
