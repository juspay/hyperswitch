use common_utils::{
    crypto::{Encryptable, GcmAes256},
    date_time,
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

use super::Identifier;
use crate::{
    errors::{CustomResult, ValidationError},
    routes::SessionState,
    types::domain::types::TypeEncryption,
};

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
        state: &SessionState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        let identifier = Identifier::User(String::from_utf8_lossy(key.peek()).to_string());
        Ok(Self {
            key: Encryptable::decrypt_via_api(state, item.key, identifier, GcmAes256)
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
