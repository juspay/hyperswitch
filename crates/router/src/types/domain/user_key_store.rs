use common_utils::{
    crypto::Encryptable,
    date_time,
    types::keymanager::{Identifier, KeyManagerState},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::type_encryption::decrypt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use crate::errors::{CustomResult, ValidationError};

#[derive(Clone, Debug, serde::Serialize)]
pub struct UserKeyStore {
    pub user_id: String,
    pub key: Encryptable<Secret<Vec<u8>>>,
    pub created_at: PrimitiveDateTime,
}

#[async_trait::async_trait]
impl super::behaviour::Conversion for UserKeyStore {
    type DstType = diesel_models::user_key_store::UserKeyStore;
    type NewDstType = diesel_models::user_key_store::UserKeyStoreNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::user_key_store::UserKeyStore {
            key: self.key.into(),
            user_id: self.user_id,
            created_at: self.created_at,
        })
    }

    async fn convert_back(
        state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_store_ref_id: common_utils::id_type::MerchantId,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let identifier = Identifier::User(item.user_id.clone());
        Ok(Self {
            key: decrypt(state, item.key, identifier, key.peek())
                .await
                .change_context(ValidationError::InvalidValue {
                    message: "Failed while decrypting customer data".to_string(),
                })?,
            user_id: item.user_id,
            created_at: item.created_at,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(diesel_models::user_key_store::UserKeyStoreNew {
            user_id: self.user_id,
            key: self.key.into(),
            created_at: date_time::now(),
        })
    }
}
