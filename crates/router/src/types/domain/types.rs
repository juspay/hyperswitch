use async_trait::async_trait;
use common_utils::{
    crypto,
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
};
use diesel_models::encryption::Encryption;
use error_stack::{IntoReport, ResultExt};
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
        /// Asynchronously encrypts the given masked data using the specified key and cryptographic algorithm.
    ///
    /// # Arguments
    ///
    /// * `masked_data` - The masked data to be encrypted
    /// * `key` - The key used for encryption
    /// * `crypt_algo` - The cryptographic algorithm to be used for encryption
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the encrypted data if successful, or a `CryptoError` if an error occurs during encryption.
    ///
    async fn encrypt(
        masked_data: Secret<String, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;

        Ok(Self::new(masked_data, encrypted_data.into()))
    }

    #[instrument(skip_all)]
        /// Asynchronously decrypts the encrypted data using the provided key and cryptographic algorithm.
    /// Returns a CustomResult containing the decrypted data on success, or a CryptoError on failure.
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        let data = crypt_algo.decode_message(key, encrypted.clone())?;

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
        /// Asynchronously encrypts the masked data using the provided key and cryptographic algorithm.
    async fn encrypt(
        masked_data: Secret<serde_json::Value, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let data = serde_json::to_vec(&masked_data.peek())
            .into_report()
            .change_context(errors::CryptoError::DecodingFailed)?;
        let encrypted_data = crypt_algo.encode_message(key, &data)?;

        Ok(Self::new(masked_data, encrypted_data.into()))
    }

    #[instrument(skip_all)]
        /// Asynchronously decrypts the given encrypted data using the specified key and encryption algorithm.
    /// Returns a CustomResult containing the decrypted data or a CryptoError if decryption fails.
    async fn decrypt(
        encrypted_data: Encryption,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted = encrypted_data.into_inner();
        let data = crypt_algo.decode_message(key, encrypted.clone())?;

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
        /// Asynchronously encrypts the masked data using the specified key and cryptographic algorithm.
    /// 
    /// # Arguments
    /// 
    /// * `masked_data` - The masked data to be encrypted.
    /// * `key` - The key used for encryption.
    /// * `crypt_algo` - The specific cryptographic algorithm to be used for encryption.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing the encrypted data wrapped in a new instance of the current type, or a `CryptoError` if an error occurs during encryption.
    /// 
    async fn encrypt(
        masked_data: Secret<Vec<u8>, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted_data = crypt_algo.encode_message(key, masked_data.peek())?;

        Ok(Self::new(masked_data, encrypted_data.into()))
    }

    #[instrument(skip_all)]
        /// Asynchronously decrypts the given encrypted data using the specified key and encryption algorithm.
    ///
    /// # Arguments
    ///
    /// * `encrypted_data` - The encrypted data to be decrypted.
    /// * `key` - The key used for decryption.
    /// * `crypt_algo` - The encryption algorithm used for decryption.
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the decrypted data if successful, or a `CryptoError` if an error occurs during decryption.
    ///
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

        /// Applies the given function `func` to the value wrapped in `self`, returning a new wrapper type `Self::OtherWrapper<V, E>`.
    /// 
    /// # Arguments
    /// * `func` - The function to apply to the value wrapped in `self`
    /// 
    /// # Returns
    /// The result of applying the given function to the value wrapped in `self`, wrapped in a new wrapper type `Self::OtherWrapper<V, E>`.
    /// 
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

        /// Asynchronously applies the given function to the value wrapped by the current wrapper, and returns a new wrapper containing the result of the function's future.
    ///
    /// # Arguments
    ///
    /// * `func` - The function to be applied to the value wrapped by the current wrapper.
    ///
    /// # Returns
    ///
    /// A new wrapper containing the result of the function's future.
    ///
    async fn async_lift<Func, F, E, W>(self, func: Func) -> Self::OtherWrapper<W, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> F + Send + Sync,
        F: futures::Future<Output = Self::OtherWrapper<W, E>> + Send,
    {
        func(self).await
    }
}

#[inline]
/// Asynchronously encrypts the given inner secret using the specified key and encryption algorithm. It records the operation time and returns a CustomResult containing the encrypted Secret if successful, or a CryptoError if an error occurs.
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
/// Asynchronously encrypts an optional Secret using the provided key, returning a CustomResult
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
/// Asynchronously decrypts the inner value using the provided key and a specified encryption algorithm, recording the operation time.
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
