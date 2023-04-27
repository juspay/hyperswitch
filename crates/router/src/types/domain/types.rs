use async_trait::async_trait;
use common_utils::{
    crypto,
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
use router_env::{instrument, tracing};
use storage_models::encryption::Encryption;

use crate::routes::metrics::{request, DECRYPTION_TIME, ENCRYPTION_TIME};

pub const TIMESTAMP: i64 = 1682425530;

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
        timestamp: i64,
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

        Ok(Self::new(masked_data, encrypted_data))
    }

    #[instrument(skip_all)]
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
        timestamp: i64,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();

        crate::logger::error!("const TIMESTAMP {}, modified_at {}", TIMESTAMP, timestamp);
        let (data, encrypted) = if timestamp < TIMESTAMP {
            (
                encrypted.clone(),
                crypt_algo.encode_message(key, &encrypted)?,
            )
        } else {
            (
                crypt_algo.decode_message(key, encrypted.clone())?,
                encrypted,
            )
        };

        crate::logger::error!("data - {:?}, encrypted - {:?}", data, encrypted,);

        let value: String = std::str::from_utf8(&data)
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
    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
        timestamp: i64,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        crate::logger::error!("const TIMESTAMP {}, modified_at {}", TIMESTAMP, timestamp);
        let (data, encrypted) = if timestamp < TIMESTAMP {
            (
                encrypted.clone(),
                crypt_algo.encode_message(key, &encrypted)?,
            )
        } else {
            (
                crypt_algo.decode_message(key, encrypted.clone())?,
                encrypted,
            )
        };

        crate::logger::error!("data - {:?}, encrypted - {:?}", data, encrypted);

        let value: serde_json::Value = serde_json::from_slice(&data)
            .into_report()
            .change_context(errors::CryptoError::DecodingFailed)?;

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

        Ok(Self::new(masked_data, encrypted_data))
    }

    #[instrument(skip_all)]
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
        timestamp: i64,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();

        let (data, encrypted) = if timestamp < TIMESTAMP {
            (
                encrypted.clone(),
                crypt_algo.encode_message(key, &encrypted)?,
            )
        } else {
            (
                crypt_algo.decode_message(key, encrypted.clone())?,
                encrypted,
            )
        };
        Ok(Self::new(data.into(), encrypted))
    }
}

pub async fn get_merchant_enc_key(
    db: &dyn crate::db::StorageInterface,
    merchant_id: impl AsRef<str>,
) -> CustomResult<Vec<u8>, crate::core::errors::StorageError> {
    let merchant_id = merchant_id.as_ref();
    let key = db
        .get_merchant_key_store_by_merchant_id(merchant_id)
        .await?
        .key
        .into_inner();
    Ok(key.expose())
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

pub async fn encrypt<E: Clone, S>(
    inner: Secret<E, S>,
    key: &[u8],
) -> CustomResult<crypto::Encryptable<Secret<E, S>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        crypto::Encryptable::encrypt(inner, key, crypto::GcmAes256 {}),
        &ENCRYPTION_TIME,
    )
    .await
}

pub async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    inner: Option<Encryption>,
    key: &[u8],
    timestamp: i64,
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        inner.async_map(|item| {
            crypto::Encryptable::decrypt(item, key, crypto::GcmAes256 {}, timestamp)
        }),
        &DECRYPTION_TIME,
    )
    .await
    .transpose()
}
