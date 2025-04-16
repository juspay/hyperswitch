use serde::{Deserialize, Serialize};
use validator::Validate;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct TokenizationRequest {
    #[validate(length(min = 1, max = 64, message = "Locker ID must be between 1 and 64 characters"))]
    pub locker_id: String,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizationResponse {
    pub id: i32,
    pub locker_id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub flag: TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TokenizationFlag {
    Enabled,
    Disabled,
} 