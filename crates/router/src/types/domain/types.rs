use async_trait::async_trait;
use common_utils::{
    crypto,
    errors::{self, CustomResult},
};
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use storage_models::encryption::Encryption;

#[async_trait]
pub trait TypeEncryption<
    T,
    V: crypto::EncodeMessage + crypto::DecodeMessage,
    S: masking::Strategy<T>,
>: Sized
{
    async fn encrypt(
        masked_data: Secret<T, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;
}

#[async_trait]
impl<
        V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
        S: masking::Strategy<String> + Send,
    > TypeEncryption<String, V, S> for crypto::Encryptable<Secret<String, S>>
{
    async fn encrypt(
        masked_data: Secret<String, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;

        Ok(Self::new(masked_data, encrypted_data))
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

        Ok(Self::new(value.into(), encrypted))
    }
}

#[async_trait]
impl<
        V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
        S: masking::Strategy<serde_json::Value> + Send,
    > TypeEncryption<serde_json::Value, V, S>
    for crypto::Encryptable<Secret<serde_json::Value, S>>
{
    async fn encrypt(
        masked_data: Secret<serde_json::Value, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let data = serde_json::to_vec(&masked_data.peek())
            .into_report()
            .change_context(errors::CryptoError::DecodingFailed)?;
        let encrypted_data = crypt_algo.encode_message(key, &data)?;

        Ok(Self::new(masked_data, encrypted_data))
    }

    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        let decrypted_data = crypt_algo.decode_message(key, encrypted.clone())?;
        let value: serde_json::Value = serde_json::from_slice(&decrypted_data)
            .into_report()
            .change_context(errors::CryptoError::DecodingFailed)?;

        Ok(Self::new(value.into(), encrypted))
    }
}

pub async fn get_key_and_algo(
    _db: &dyn crate::db::StorageInterface,
    _merchant_id: String,
) -> CustomResult<Vec<u8>, crate::core::errors::StorageError> {
    Ok(Vec::new())
}

pub trait Lift<U: Clone> {
    type SelfWrapper<T>;
    type OtherWrapper<T, E>
    where
        T: Clone;

    fn lift<Func, E>(self, func: Func) -> Self::OtherWrapper<U, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> Self::OtherWrapper<U, E>;
}

impl<U: Clone, S: masking::Strategy<U> + Send> Lift<Secret<U, S>> for Option<Secret<U, S>> {
    type SelfWrapper<T> = Option<T>;
    type OtherWrapper<T: Clone, E> = CustomResult<Option<crypto::Encryptable<T>>, E>;

    fn lift<Func, E>(self, func: Func) -> Self::OtherWrapper<Secret<U, S>, E>
    where
        Func: Fn(Self::SelfWrapper<Secret<U, S>>) -> Self::OtherWrapper<Secret<U, S>, E>,
    {
        func(self)
    }
}

#[async_trait]
pub trait AsyncLift<U: Clone> {
    type SelfWrapper<T>;
    type OtherWrapper<T: Clone, E>;

    async fn async_lift<Func, F, E>(self, func: Func) -> Self::OtherWrapper<U, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> F + Send + Sync,
        F: futures::Future<Output = Self::OtherWrapper<U, E>> + Send;
}

#[async_trait]
impl<U: Clone, V: Lift<U> + Lift<U, SelfWrapper<U> = V> + Send> AsyncLift<U> for V {
    type SelfWrapper<T> = <V as Lift<U>>::SelfWrapper<T>;
    type OtherWrapper<T: Clone, E> = <V as Lift<U>>::OtherWrapper<T, E>;

    async fn async_lift<Func, F, E>(self, func: Func) -> Self::OtherWrapper<U, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> F + Send + Sync,
        F: futures::Future<Output = Self::OtherWrapper<U, E>> + Send,
    {
        func(self).await
    }
}
