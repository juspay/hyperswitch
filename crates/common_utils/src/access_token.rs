//! Commonly used utilities for access token

use std::fmt::Display;

use crate::id_type;

/// Create a key for fetching the access token from redis
pub fn create_access_token_key(
    merchant_id: &id_type::MerchantId,
    merchant_connector_id_or_connector_name: impl Display,
) -> String {
    merchant_id.get_access_token_key(merchant_connector_id_or_connector_name)
}
