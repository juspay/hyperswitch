use common_utils::pii;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use common_utils::id_type::GlobalTokenId;

use crate::types;
use common_utils::consts::MAX_LOCKER_ID_LENGTH;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct Tokenization {
    pub id: GlobalTokenId,
    pub merchant_id: String,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub flag: types::TokenizationFlag,
    pub version: types::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct TokenizationNew {
    pub merchant_id: String,
    pub locker_id: String,
    pub flag: types::TokenizationFlag,
    pub version: types::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenizationResponse {
    pub token: String,
    pub message: String,
}

impl From<Tokenization> for TokenizationResponse {
    fn from(value: Tokenization) -> Self {
        Self {
            id: value.id,
            created_at: value.created_at.assume_utc().unix_timestamp(),
            flag: value.flag,
        }
    }
} 