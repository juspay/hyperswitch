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
