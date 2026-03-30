use common_utils::{
    crypto::Encryptable,
    date_time,
    errors::{CustomResult, ValidationError},
    type_name,
    types::keymanager::{self, KeyManagerState},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    merchant_key_store::MerchantKeyStore,
    type_encryption::{crypto_operation, CryptoOperation},
};
use hyperswitch_masking::{PeekInterface, Secret};

use crate::behaviour::Conversion;

#[async_trait::async_trait]
impl Conversion for MerchantKeyStore {
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

        let decryption_operation = if state.use_legacy_key_store_decryption {
            CryptoOperation::Decrypt(item.key)
        } else {
            CryptoOperation::DecryptLocally(item.key)
        };

        Ok(Self {
            key: crypto_operation(
                state,
                type_name!(Self::DstType),
                decryption_operation,
                identifier,
                key.peek(),
            )
            .await
            .and_then(|val| val.try_into_operation())
            .change_context(ValidationError::InvalidValue {
                message: "Failed while decrypting merchant key store".to_string(),
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
