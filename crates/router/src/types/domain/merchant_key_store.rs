use common_utils::crypto::{Encryptable, GcmAes256};
use error_stack::ResultExt;
use masking::Secret;

use crate::{
    db::StorageInterface,
    errors::{CustomResult, ValidationError},
    types::domain::types::TypeEncryption,
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct MerchantKeyStore {
    pub id: Option<i32>,
    pub merchant_id: String,
    pub key: Encryptable<Secret<Vec<u8>>>,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for MerchantKeyStore {
    type DstType = storage_models::merchant_key_store::MerchantKeyStore;
    type NewDstType = storage_models::merchant_key_store::MerchantKeyStoreNew;
    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(storage_models::merchant_key_store::MerchantKeyStore {
            id: self.id.ok_or(ValidationError::MissingRequiredField {
                field_name: "id".to_string(),
            })?,
            key: self.key.into(),
            merchant_id: self.merchant_id,
        })
    }

    async fn convert_back(
        item: Self::DstType,
        db: &dyn StorageInterface,
        _merchant_id: &str,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let key = &db.get_master_key();
        Ok(Self {
            id: Some(item.id),
            key: Encryptable::decrypt(item.key, key, GcmAes256 {})
                .await
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting customer data".to_string(),
                })?,
            merchant_id: item.merchant_id,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(storage_models::merchant_key_store::MerchantKeyStoreNew {
            merchant_id: self.merchant_id,
            key: self.key.into(),
        })
    }
}
