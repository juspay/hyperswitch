use async_trait::async_trait;
#[allow(unused_imports)]
use common_utils::{
    crypto,
    errors::{self, CustomResult},
    ext_traits::{AsyncExt, BytesExt},
};
use diesel_models::encryption::Encryption;
use error_stack::ResultExt;
#[allow(unused_imports)]
use masking::{ExposeInterface, StrongSecret};
use masking::{PeekInterface, Secret};
use router_env::{instrument, tracing};

#[allow(unused_imports)]
use super::{
    DecryptDataRequest, DecryptDataResponse, DecryptedData, EncryptDataRequest,
    EncryptDataResponse, EncryptedData, Identifier, Version,
};
#[allow(unused_imports)]
use crate::{
    encryption::call_encryption_service,
    routes::{
        metrics::{request, DECRYPTION_TIME, ENCRYPTION_TIME},
        SessionState,
    },
};

#[async_trait]
pub trait TypeEncryption<
    T,
    V: crypto::EncodeMessage + crypto::DecodeMessage,
    S: masking::Strategy<T>,
>: Sized
{
    async fn encrypt_via_api(
        state: &SessionState,
        masked_data: Secret<T, S>,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;

    async fn decrypt_via_api(
        state: &SessionState,
        encrypted_data: Encryption,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;

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
    #[allow(unused_variables)]
    async fn encrypt_via_api(
        state: &SessionState,
        masked_data: Secret<String, S>,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let request_body = EncryptDataRequest {
                data: DecryptedData::from_data(StrongSecret::new(
                    masked_data.clone().expose().as_bytes().to_vec(),
                )),
                identifier: identifier.clone(),
            };
            let result = call_encryption_service(state, "encrypt", request_body).await;
            match result {
                Ok(response) => match response {
                    Ok(encrypt_response) => {
                        let encrypt_object = encrypt_response
                            .response
                            .parse_struct::<EncryptDataResponse>("EncryptDataResponse")
                            .change_context(errors::CryptoError::EncodingFailed);
                        match encrypt_object {
                            Ok(encrypted) => Ok(Self::new(
                                masked_data,
                                encrypted.data.data.peek().clone().into(),
                            )),
                            Err(_) => {
                                Self::encrypt(
                                    masked_data,
                                    identifier.inner().as_bytes(),
                                    crypt_algo,
                                )
                                .await
                            }
                        }
                    }
                    Err(_) => {
                        Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
                    }
                },
                Err(_) => {
                    Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
                }
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &SessionState,
        encrypted_data: Encryption,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let request_body = DecryptDataRequest {
                data: EncryptedData {
                    data: StrongSecret::new(encrypted_data.clone().into_inner().expose()),
                    version: Version::from("v1".to_string()),
                },
                identifier: identifier.clone(),
            };
            let result = call_encryption_service(state, "decrypt", request_body).await;
            match result {
                Ok(response) => match response {
                    Ok(decrypt_response) => {
                        let decrypt_object = decrypt_response
                            .response
                            .parse_struct::<DecryptDataResponse>("DecryptDataResponse")
                            .change_context(errors::CryptoError::DecodingFailed);
                        match decrypt_object {
                            Ok(decrypted) => Ok(Self::new(
                                String::from_utf8_lossy(decrypted.data.inner().peek())
                                    .to_string()
                                    .into(),
                                encrypted_data.into_inner(),
                            )),
                            Err(_) => {
                                Self::decrypt(
                                    encrypted_data,
                                    identifier.inner().as_bytes(),
                                    crypt_algo,
                                )
                                .await
                            }
                        }
                    }
                    Err(_) => {
                        Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo)
                            .await
                    }
                },
                Err(_) => {
                    Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo).await
                }
            }
        }
    }

    async fn encrypt(
        masked_data: Secret<String, S>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;
        Ok(Self::new(masked_data, encrypted_data.into()))
    }

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
    #[allow(unused_variables)]
    async fn encrypt_via_api(
        state: &SessionState,
        masked_data: Secret<serde_json::Value, S>,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let request_body = EncryptDataRequest {
                data: DecryptedData::from_data(StrongSecret::new(
                    masked_data.clone().expose().to_string().as_bytes().to_vec(),
                )),
                identifier: identifier.clone(),
            };
            let result = call_encryption_service(state, "encrypt", request_body).await;
            match result {
                Ok(response) => match response {
                    Ok(encrypt_response) => {
                        let encrypt_object = encrypt_response
                            .response
                            .parse_struct::<EncryptDataResponse>("EncryptDataResponse")
                            .change_context(errors::CryptoError::EncodingFailed);
                        match encrypt_object {
                            Ok(encrypted) => Ok(Self::new(
                                masked_data,
                                encrypted.data.data.peek().clone().into(),
                            )),
                            Err(_) => {
                                Self::encrypt(
                                    masked_data,
                                    identifier.inner().as_bytes(),
                                    crypt_algo,
                                )
                                .await
                            }
                        }
                    }
                    Err(_) => {
                        Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
                    }
                },
                Err(_) => {
                    Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
                }
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &SessionState,
        encrypted_data: Encryption,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let request_body = DecryptDataRequest {
                data: EncryptedData {
                    data: StrongSecret::new(encrypted_data.clone().into_inner().expose()),
                    version: Version::from("v1".to_string()),
                },
                identifier: identifier.clone(),
            };
            let result = call_encryption_service(state, "decrypt", request_body).await;
            match result {
                Ok(response) => match response {
                    Ok(decrypt_response) => {
                        let decrypt_object = decrypt_response
                            .response
                            .parse_struct::<DecryptDataResponse>("DecryptDataResponse")
                            .change_context(errors::CryptoError::DecodingFailed);
                        match decrypt_object {
                            Ok(decrypted) => {
                                let value: Result<serde_json::Value, serde_json::Error> =
                                    serde_json::from_slice(decrypted.data.inner().peek());
                                match value {
                                    Ok(val) => {
                                        Ok(Self::new(val.into(), encrypted_data.into_inner()))
                                    }
                                    Err(_) => {
                                        Self::decrypt(
                                            encrypted_data,
                                            identifier.inner().as_bytes(),
                                            crypt_algo,
                                        )
                                        .await
                                    }
                                }
                            }
                            Err(_) => {
                                Self::decrypt(
                                    encrypted_data,
                                    identifier.inner().as_bytes(),
                                    crypt_algo,
                                )
                                .await
                            }
                        }
                    }
                    Err(_) => {
                        Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo)
                            .await
                    }
                },
                Err(_) => {
                    Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo).await
                }
            }
        }
    }

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
    #[allow(unused_variables)]
    async fn encrypt_via_api(
        state: &SessionState,
        masked_data: Secret<Vec<u8>, S>,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let request_body = EncryptDataRequest {
                data: DecryptedData::from_data(StrongSecret::new(
                    masked_data.clone().expose().to_vec(),
                )),
                identifier: identifier.clone(),
            };
            let result = call_encryption_service(state, "encrypt", request_body).await;
            match result {
                Ok(response) => match response {
                    Ok(encrypt_response) => {
                        let encrypt_object = encrypt_response
                            .response
                            .parse_struct::<EncryptDataResponse>("EncryptDataResponse")
                            .change_context(errors::CryptoError::EncodingFailed);
                        match encrypt_object {
                            Ok(encrypted) => Ok(Self::new(
                                masked_data.clone(),
                                encrypted.data.data.peek().clone().into(),
                            )),
                            Err(_) => {
                                Self::encrypt(
                                    masked_data,
                                    identifier.inner().as_bytes(),
                                    crypt_algo,
                                )
                                .await
                            }
                        }
                    }
                    Err(_) => {
                        Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
                    }
                },
                Err(_) => {
                    Self::encrypt(masked_data, identifier.inner().as_bytes(), crypt_algo).await
                }
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &SessionState,
        encrypted_data: Encryption,
        identifier: Identifier,
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let request_body = DecryptDataRequest {
                data: EncryptedData {
                    data: StrongSecret::new(encrypted_data.clone().into_inner().expose()),
                    version: Version::from("v1".to_string()),
                },
                identifier: identifier.clone(),
            };
            let result = call_encryption_service(state, "decrypt", request_body).await;
            match result {
                Ok(response) => match response {
                    Ok(decrypt_response) => {
                        let decrypt_object = decrypt_response
                            .response
                            .parse_struct::<DecryptDataResponse>("DecryptDataResponse")
                            .change_context(errors::CryptoError::DecodingFailed);
                        match decrypt_object {
                            Ok(decrypted) => Ok(Self::new(
                                decrypted.data.inner().peek().clone().into(),
                                encrypted_data.into_inner(),
                            )),
                            Err(_) => {
                                Self::decrypt(
                                    encrypted_data,
                                    identifier.inner().as_bytes(),
                                    crypt_algo,
                                )
                                .await
                            }
                        }
                    }
                    Err(_) => {
                        Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo)
                            .await
                    }
                },
                Err(_) => {
                    Self::decrypt(encrypted_data, identifier.inner().as_bytes(), crypt_algo).await
                }
            }
        }
    }

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
    state: &SessionState,
    inner: Secret<E, S>,
    identifier: Identifier,
) -> CustomResult<crypto::Encryptable<Secret<E, S>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        crypto::Encryptable::encrypt_via_api(state, inner, identifier, crypto::GcmAes256),
        &ENCRYPTION_TIME,
        &[],
    )
    .await
}

#[inline]
pub async fn encrypt_optional<E: Clone, S>(
    state: &SessionState,
    inner: Option<Secret<E, S>>,
    identifier: Identifier,
) -> CustomResult<Option<crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    Secret<E, S>: Send,
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    inner
        .async_map(|f| encrypt(state, f, identifier))
        .await
        .transpose()
}

#[inline]
pub async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    state: &SessionState,
    inner: Option<Encryption>,
    identifier: Identifier,
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        inner.async_map(|item| {
            crypto::Encryptable::decrypt_via_api(state, item, identifier, crypto::GcmAes256)
        }),
        &DECRYPTION_TIME,
        &[],
    )
    .await
    .transpose()
}
