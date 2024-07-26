use common_utils::{
    crypto::{self, DecodeMessage, Encryptable},
    custom_serde, date_time,
    encryption::Encryption,
    errors::{self, CustomResult, ValidationError},
    types::keymanager::{self, KeyManagerState},
};
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use time::PrimitiveDateTime;

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
        _state: &KeyManagerState,
        item: Self::DstType,
        key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError>
    where
        Self: Sized,
    {
        // let identifier = keymanager::Identifier::Merchant(item.merchant_id.clone());
        Ok(Self {
            //key: decrypt(state, item.key, identifier, key.peek())
            // Replace this method call with above commented line while deprecating merchant key store
            key: decrypt_merchant_key_store(item.key, key.peek())
                .await
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

/// This is a temprory function to decrypt the merchant key store within application
/// Since key in merchant key store is encrypted with master key in the application, decrypt can't be done with key manager service.
/// Once application merchant key store is deprecated, this method should be removed.
async fn decrypt_merchant_key_store(
    encrypted_data: Encryption,
    key: &[u8],
) -> CustomResult<Encryptable<Secret<Vec<u8>>>, errors::CryptoError> {
    metrics::MERCHANT_KEY_STORE_APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
    let encrypted = encrypted_data.into_inner();
    let data = crypto::GcmAes256.decode_message(key, encrypted.clone())?;
    Ok(Encryptable::new(data.into(), encrypted))
}

mod metrics {
    use router_env::{counter_metric, global_meter, metrics_context, once_cell};

    metrics_context!(CONTEXT);
    global_meter!(GLOBAL_METER, "ROUTER_API");

    // Encryption and Decryption metrics
    counter_metric!(
        MERCHANT_KEY_STORE_APPLICATION_DECRYPTION_COUNT,
        GLOBAL_METER
    );
}
