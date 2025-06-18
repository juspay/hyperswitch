use common_enums;
use common_utils::{
    self,
    errors::{CustomResult, ValidationError},
    id_type, pii,
    types::{keymanager, MinorUnit},
};
use diesel_models::tokenization;
use masking::{ExposeInterface, Secret};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{merchant_key_store::MerchantKeyStore, types};

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

#[async_trait::async_trait]
impl super::behaviour::Conversion for Tokenization {
    type DstType = diesel_models::tokenization::Tokenization;
    type NewDstType = diesel_models::tokenization::Tokenization;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::tokenization::Tokenization {
            id: self.id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            locker_id: self.locker_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            flag: self.flag,
        })
    }

    async fn convert_back(
        _state: &keymanager::KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: item.id,
            merchant_id: item.merchant_id,
            customer_id: item.customer_id,
            locker_id: item.locker_id,
            created_at: item.created_at,
            updated_at: item.updated_at,
            flag: item.flag,
            version: item.version,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::tokenization::Tokenization {
            id: self.id,
            merchant_id: self.merchant_id,
            customer_id: self.customer_id,
            locker_id: self.locker_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            version: self.version,
            flag: self.flag,
        })
    }
}
