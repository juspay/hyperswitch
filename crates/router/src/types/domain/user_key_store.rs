use common_utils::{
    crypto::{self, DecodeMessage, Encryptable},
    date_time,
    encryption::Encryption,
    errors,
    types::keymanager::{Identifier, KeyManagerState},
};
use error_stack::ResultExt;
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
        _state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        // let identifier = Identifier::User(item.user_id.clone());
        Ok(Self {
            // key: decrypt(state, item.key, identifier, key.peek())
            // Replace this method call with above commented line while deprecating user key store
            key: decrypt_user_key_store(item.key, key.peek())
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

/// This is a temprory function to decrypt the user key store within application
/// Since key in user key store is encrypted with master key in the application, decrypt can't be done with key manager service.
/// Once application user key store is deprecated, this method should be removed.
async fn decrypt_user_key_store(
    encrypted_data: Encryption,
    key: &[u8],
) -> CustomResult<Encryptable<Secret<Vec<u8>>>, errors::CryptoError> {
    metrics::USER_KEY_STORE_APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
    let encrypted = encrypted_data.into_inner();
    let data = crypto::GcmAes256.decode_message(key, encrypted.clone())?;
    Ok(Encryptable::new(data.into(), encrypted))
}

mod metrics {
    use router_env::{counter_metric, global_meter, metrics_context, once_cell};

    metrics_context!(CONTEXT);
    global_meter!(GLOBAL_METER, "ROUTER_API");

    // Encryption and Decryption metrics
    counter_metric!(USER_KEY_STORE_APPLICATION_DECRYPTION_COUNT, GLOBAL_METER);
}
