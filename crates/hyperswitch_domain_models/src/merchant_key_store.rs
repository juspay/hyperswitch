use common_utils::{crypto::Encryptable, custom_serde, errors::CustomResult};
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantKeyStore {
    pub merchant_id: common_utils::id_type::MerchantId,
    pub key: Encryptable<Secret<Vec<u8>>>,
    #[serde(with = "custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}

#[async_trait::async_trait]
pub trait MerchantKeyStoreInterface {
    type Error;
    async fn insert_merchant_key_store(
        &self,
        merchant_key_store: MerchantKeyStore,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<MerchantKeyStore, Self::Error>;

    async fn get_merchant_key_store_by_merchant_id(
        &self,
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
        merchant_ids: Vec<common_utils::id_type::MerchantId>,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Vec<MerchantKeyStore>, Self::Error>;

    async fn get_all_key_stores(
        &self,
        key: &Secret<Vec<u8>>,
        from: u32,
        to: u32,
    ) -> CustomResult<Vec<MerchantKeyStore>, Self::Error>;
}
