use common_enums;
use common_utils::{
    self,
    errors::{CustomResult, ValidationError},
    types::keymanager,
};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tokenization {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub locker_id: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub flag: common_enums::TokenizationFlag,
    pub version: common_enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenizationNew {
    pub id: common_utils::id_type::GlobalTokenId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub customer_id: common_utils::id_type::GlobalCustomerId,
    pub locker_id: String,
    pub flag: common_enums::TokenizationFlag,
    pub version: common_enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenizationResponse {
    pub token: String,
    pub created_at: PrimitiveDateTime,
    pub flag: common_enums::TokenizationFlag,
}

impl From<Tokenization> for TokenizationResponse {
    fn from(value: Tokenization) -> Self {
        Self {
            token: value.id.get_string_repr().to_string(),
            created_at: value.created_at,
            flag: value.flag,
        }
    }
}
