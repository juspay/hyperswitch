//! Module for tokenization-related functionality
//! 
//! This module provides types and functions for handling tokenized payment data,
//! including response structures and token generation utilities.

use serde::{Deserialize, Serialize};
use diesel;
use common_enums::ApiVersion;
use time::PrimitiveDateTime;
use crate::{
    consts::{MAX_LOCKER_ID_LENGTH, TOKEN_LENGTH},
    id_type::GlobalTokenId,
};

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
/// Response structure for tokenization operations
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizationResponse {
    /// Unique identifier for the tokenized data
    pub id: GlobalTokenId,
    /// Reference to the vault/locker where the actual data is stored
    pub locker_id: String,
    /// Timestamp when the token was created
    pub created_at: PrimitiveDateTime,
    /// Timestamp when the token was last updated
    pub updated_at: PrimitiveDateTime,
    /// Current status of the token
    pub flag: TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
/// Generates a new token string
/// 
/// # Returns
/// A randomly generated token string of length `TOKEN_LENGTH`
pub fn generate_token() -> String {
    use nanoid::nanoid;
    nanoid!(TOKEN_LENGTH)
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
/// Enum representing the status of a tokenized payment method
#[derive(Debug, Clone, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TokenizationFlag {
    /// Token is active and can be used for payments
    Enabled,
    /// Token is inactive and cannot be used for payments
    Disabled,
}