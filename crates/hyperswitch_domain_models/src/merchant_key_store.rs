use common_utils::{
    crypto::Encryptable,
    custom_serde, date_time,
    errors::{CustomResult, ValidationError},
    type_name,
    types::keymanager::{self, KeyManagerState},
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use crate::type_encryption::{crypto_operation, CryptoOperation};

#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantKeyStore {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub key: Encryptable<Secret<Vec<u8>>>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for MerchantKeyStore {
    type DstType = diesel_models::merchant_key_store::MerchantKeyStore;
    type NewDstType = diesel_models::merchant_key_store::MerchantKeyStoreNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::merchant_key_store::MerchantKeyStore {
            key: self.key.into(),
            merchant_id: self.merchant_id,
            created_at: self.created_at,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let identifier = keymanager::Identifier::Merchant(item.merchant_id.clone());

        Ok(Self {
            key: crypto_operation(
                state,
                type_name!(Self::DstType),
                CryptoOperation::Decrypt(item.key),
                identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting customer data".to_string(),
            })?,
            merchant_id: item.merchant_id,
            created_at: item.created_at,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::merchant_key_store::MerchantKeyStoreNew {
            merchant_id: self.merchant_id,
            key: self.key.into(),
            created_at: date_time::now(),
        })
    }
}

#[async_trait::async_trait]
pub trait MerchantKeyStoreInterface {
    type Error;
    async fn insert_merchant_key_store(
        &self,
        state: &KeyManagerState,
        merchant_key_store: MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<MerchantKeyStore, Self::Error>;

    async fn get_merchant_key_store_by_merchant_id(
        &self,
        state: &KeyManagerState,
        merchant_id: &common_utils::id_type::MerchantId,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<MerchantKeyStore, Self::Error>;

    async fn delete_merchant_key_store_by_merchant_id(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> CustomResult<bool, Self::Error>;

    #[cfg(feature = "olap")]
    async fn list_multiple_key_stores(
        &self,
        state: &KeyManagerState,
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<MerchantKeyStore>, Self::Error>;

    async fn get_all_key_stores(
        &self,
        state: &KeyManagerState,
        key: &Secret<Vec<u8>>,
        from: u32,
        to: u32,
    ) -> CustomResult<Vec<MerchantKeyStore>, Self::Error>;
}
