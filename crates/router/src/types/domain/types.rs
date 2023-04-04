use std::ops::Deref;

use async_trait::async_trait;
use common_utils::{
    crypto,
    errors::{self, CustomResult},
};
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use storage_models::encryption::Encryption;

#[derive(Debug)]
pub struct Encryptable<T> {
    inner: T,
    encrypted: Vec<u8>,
}

#[async_trait]
pub trait TypeEncryption<T, V: crypto::EncodeMessage + crypto::DecodeMessage>: Sized {
    async fn encrypt(
        masked_data: Secret<T>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;
}

impl<T> Deref for Encryptable<Secret<T>> {
    type Target = Secret<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> From<Encryptable<T>> for Encryption {
    fn from(value: Encryptable<T>) -> Self {
        Self::new(value.encrypted)
    }
}

#[async_trait]
impl<V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static> TypeEncryption<String, V>
    for Encryptable<Secret<String>>
{
    async fn encrypt(
        masked_data: Secret<String>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;

        Ok(Self {
            inner: masked_data,
            encrypted: encrypted_data,
        })
    }

    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        let decrypted_data = crypt_algo.decode_message(key, encrypted.clone())?;
        let value: String = std::str::from_utf8(&decrypted_data)
            .into_report()
            .change_context(errors::CryptoError::DecodingFailed)?
            .to_string();

        Ok(Self {
            inner: value.into(),
            encrypted,
        })
    }
}
