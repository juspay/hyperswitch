use async_trait::async_trait;
use common_utils::{
    crypto,
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
};
use error_stack::{IntoReport, ResultExt};
use masking::{PeekInterface, Secret};
use storage_models::encryption::Encryption;

use crate::core::errors as core_errors;

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

pub async fn get_key_and_algo(
    _db: &dyn crate::db::StorageInterface,
    _merchant_id: String,
) -> CustomResult<Vec<u8>, core_errors::StorageError> {
    Ok(Vec::new())
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

pub(crate) async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    inner: Option<Encryption>,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    inner
        .async_map(|item| crypto::Encryptable::decrypt(item, key, crypto::GcmAes256 {}))
        .await
        .transpose()
}
