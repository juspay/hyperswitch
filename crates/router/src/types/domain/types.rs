use async_trait::async_trait;
use common_utils::{
    crypto,
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
};
use diesel_models::encryption::Encryption;
use error_stack::ResultExt;
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};

use crate::routes::metrics::{request, DECRYPTION_TIME, ENCRYPTION_TIME};

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
    #[instrument(skip_all)]
    async fn encrypt(
        masked_data: Secret<String, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;

        Ok(Self::new(masked_data, encrypted_data.into()))
    }

    #[instrument(skip_all)]
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        let data = crypt_algo.decode_message(key, encrypted.clone())?;

        let value: String = std::str::from_utf8(&data)
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
    #[instrument(skip_all)]
    async fn encrypt(
        masked_data: Secret<serde_json::Value, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let data = serde_json::to_vec(&masked_data.peek())
            .change_context(errors::CryptoError::DecodingFailed)?;
        let encrypted_data = crypt_algo.encode_message(key, &data)?;

        Ok(Self::new(masked_data, encrypted_data.into()))
    }

    #[instrument(skip_all)]
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        let data = crypt_algo.decode_message(key, encrypted.clone())?;

        let value: serde_json::Value =
            serde_json::from_slice(&data).change_context(errors::CryptoError::DecodingFailed)?;

        Ok(Self::new(value.into(), encrypted))
    }
}

#[async_trait]
impl<
        V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
        S: masking::Strategy<Vec<u8>> + Send,
    > TypeEncryption<Vec<u8>, V, S> for crypto::Encryptable<Secret<Vec<u8>, S>>
{
    #[instrument(skip_all)]
    async fn encrypt(
        masked_data: Secret<Vec<u8>, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted_data = crypt_algo.encode_message(key, masked_data.peek())?;

        Ok(Self::new(masked_data, encrypted_data.into()))
    }

    #[instrument(skip_all)]
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        let data = crypt_algo.decode_message(key, encrypted.clone())?;

        Ok(Self::new(data.into(), encrypted))
    }
}

pub trait Lift<U> {
    type SelfWrapper<T>;
    type OtherWrapper<T, E>;

    fn lift<Func, E, V>(self, func: Func) -> Self::OtherWrapper<V, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> Self::OtherWrapper<V, E>;
}

impl<U> Lift<U> for Option<U> {
    type SelfWrapper<T> = Option<T>;
    type OtherWrapper<T, E> = CustomResult<Option<T>, E>;

    fn lift<Func, E, V>(self, func: Func) -> Self::OtherWrapper<V, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> Self::OtherWrapper<V, E>,
    {
        func(self)
    }
}

#[async_trait]
pub trait AsyncLift<U> {
    type SelfWrapper<T>;
    type OtherWrapper<T, E>;

    async fn async_lift<Func, F, E, V>(self, func: Func) -> Self::OtherWrapper<V, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> F + Send + Sync,
        F: futures::Future<Output = Self::OtherWrapper<V, E>> + Send;
}

#[async_trait]
impl<U, V: Lift<U> + Lift<U, SelfWrapper<U> = V> + Send> AsyncLift<U> for V {
    type SelfWrapper<T> = <V as Lift<U>>::SelfWrapper<T>;
    type OtherWrapper<T, E> = <V as Lift<U>>::OtherWrapper<T, E>;

    async fn async_lift<Func, F, E, W>(self, func: Func) -> Self::OtherWrapper<W, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> F + Send + Sync,
        F: futures::Future<Output = Self::OtherWrapper<W, E>> + Send,
    {
        func(self).await
    }
}

#[inline]
pub async fn encrypt<E: Clone, S>(
    inner: Secret<E, S>,
    key: &[u8],
) -> CustomResult<crypto::Encryptable<Secret<E, S>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        crypto::Encryptable::encrypt(inner, key, crypto::GcmAes256),
        &ENCRYPTION_TIME,
        &[],
    )
    .await
}

#[inline]
pub async fn encrypt_optional<E: Clone, S>(
    inner: Option<Secret<E, S>>,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    Secret<E, S>: Send,
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    inner.async_map(|f| encrypt(f, key)).await.transpose()
}

#[inline]
pub async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    inner: Option<Encryption>,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        inner.async_map(|item| crypto::Encryptable::decrypt(item, key, crypto::GcmAes256)),
        &DECRYPTION_TIME,
        &[],
    )
    .await
    .transpose()
}
