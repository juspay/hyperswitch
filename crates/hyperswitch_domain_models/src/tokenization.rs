use common_utils::pii;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use validator::Validate;

use crate::types;
use common_utils::consts::MAX_LOCKER_ID_LENGTH;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct Tokenization {
    pub id: i32,
    #[validate(length(min = 1, max = "MAX_LOCKER_ID_LENGTH"))]
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub flag: types::TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct TokenizationNew {
    #[validate(length(min = 1, max = "MAX_LOCKER_ID_LENGTH"))]
    pub locker_id: String,
    pub flag: types::TokenizationFlag,
} 