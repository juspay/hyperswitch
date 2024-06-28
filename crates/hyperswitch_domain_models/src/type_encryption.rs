use async_trait::async_trait;
use common_utils::{
    crypto,
    encryption::Encryption,
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
    metrics::utils::record_operation_time,
    types::keymanager::{Identifier, KeyManagerState, DEFAULT_KEY},
};
#[cfg(feature = "encryption_service")]
use common_utils::{
    keymanager::call_encryption_service,
    types::keymanager::{
        DecryptDataRequest, DecryptDataResponse, EncryptDataRequest, EncryptDataResponse,
    },
};
use error_stack::ResultExt;
#[allow(unused_imports)]
use masking::{ExposeInterface, StrongSecret};
use masking::{PeekInterface, Secret};
#[allow(unused_imports)]
use rdkafka::message::ToBytes;
#[allow(unused_imports)]
use router_env::{instrument, logger, tracing};
use rustc_hash::FxHashMap;

#[async_trait]
pub trait TypeEncryption<
    T,
    V: crypto::EncodeMessage + crypto::DecodeMessage,
    S: masking::Strategy<T>,
>: Sized
{
    async fn encrypt_via_api(
        state: &KeyManagerState,
        masked_data: Secret<T, S>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;

    async fn decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: Encryption,
        identifier: Identifier,
        key: &[u8],
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

    async fn batch_encrypt_via_api(
        state: &KeyManagerState,
        masked_data: FxHashMap<String, Secret<T, S>>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

    async fn batch_decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: FxHashMap<String, Encryption>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

    async fn batch_encrypt(
        masked_data: FxHashMap<String, Secret<T, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

    async fn batch_decrypt(
        encrypted_data: FxHashMap<String, Encryption>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;
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
        state: &KeyManagerState,
        masked_data: Secret<String, S>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::encrypt(masked_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match result {
                Ok(encrypted_data) => encrypted_data
                    .data
                    .0
                    .get(DEFAULT_KEY)
                    .map(|ed| Self::new(masked_data.clone(), ed.data.peek().clone().into())),
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => {
                    logger::info!("Fall back to Application Encryption");
                    Self::encrypt(masked_data, key, crypt_algo).await
                }
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: Encryption,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::decrypt(encrypted_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match result {
                Ok(decrypted_data) => match decrypted_data.data.0.get(DEFAULT_KEY) {
                    Some(data) => {
                        let inner = String::from_utf8(data.clone().inner().peek().clone())
                            .change_context(errors::CryptoError::DecodingFailed)?
                            .into();
                        Ok(Self::new(inner, encrypted_data.clone().into_inner()))
                    }
                    None => Err(errors::CryptoError::DecodingFailed),
                },
                Err(_) => Err(errors::CryptoError::DecodingFailed),
            };

            match decrypted {
                Ok(de) => Ok(de),
                Err(_) => {
                    logger::info!("Fall back to Application Decryption");
                    Self::decrypt(encrypted_data, key, crypt_algo).await
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

    #[allow(unused_variables)]
    async fn batch_encrypt_via_api(
        state: &KeyManagerState,
        masked_data: FxHashMap<String, Secret<String, S>>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::batch_encrypt(masked_data, key, crypt_algo).await
        }

        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(encrypted_data) => {
                    let mut encrypted = FxHashMap::default();
                    for (k, v) in encrypted_data.data.0.iter() {
                        masked_data.get(k).map(|inner| {
                            encrypted.insert(
                                k.clone(),
                                Self::new(inner.clone(), v.data.peek().clone().into()),
                            )
                        });
                    }
                    Ok(encrypted)
                }
                Err(_) => {
                    logger::info!("Fall back to Application Encryption");
                    Self::batch_encrypt(masked_data, key, crypt_algo).await
                }
            }
        }
    }

    #[allow(unused_variables)]
    async fn batch_decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: FxHashMap<String, Encryption>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::batch_decrypt(encrypted_data, key, crypt_algo).await
        }

        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(decrypted_data) => {
                    let mut decrypted = FxHashMap::default();
                    for (k, v) in decrypted_data.data.0.iter() {
                        let inner = String::from_utf8(v.clone().inner().peek().clone())
                            .change_context(errors::CryptoError::DecodingFailed)?;
                        encrypted_data.get(k).map(|encrypted| {
                            decrypted.insert(
                                k.clone(),
                                Self::new(inner.into(), encrypted.clone().into_inner()),
                            )
                        });
                    }
                    Ok(decrypted)
                }
                Err(_) => {
                    logger::info!("Fall back to Application Decryption");
                    Self::batch_decrypt(encrypted_data, key, crypt_algo).await
                }
            }
        }
    }

    async fn batch_encrypt(
        masked_data: FxHashMap<String, Secret<String, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
        for (k, v) in masked_data {
            encrypted.insert(
                k,
                Self::new(
                    v.clone(),
                    crypt_algo.encode_message(key, v.peek().as_bytes())?.into(),
                ),
            );
        }
        Ok(encrypted)
    }

    async fn batch_decrypt(
        encrypted_data: FxHashMap<String, Encryption>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
        for (k, v) in encrypted_data {
            let data = crypt_algo.decode_message(key, v.clone().into_inner())?;

            let value: String = std::str::from_utf8(&data)
                .change_context(errors::CryptoError::DecodingFailed)?
                .to_string();
            decrypted.insert(k, Self::new(value.into(), v.into_inner()));
        }
        Ok(decrypted)
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
        state: &KeyManagerState,
        masked_data: Secret<serde_json::Value, S>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::encrypt(masked_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match result {
                Ok(encrypted_data) => encrypted_data.data.0.get(DEFAULT_KEY).map(|encrypted| {
                    Self::new(masked_data.clone(), encrypted.data.peek().clone().into())
                }),
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => {
                    logger::info!("Fall back to Application Encryption");
                    Self::encrypt(masked_data, key, crypt_algo).await
                }
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: Encryption,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::decrypt(encrypted_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match result {
                Ok(decrypted_data) => match decrypted_data.data.0.get(DEFAULT_KEY) {
                    Some(data) => {
                        let value: serde_json::Value =
                            serde_json::from_slice(data.clone().inner().peek())
                                .change_context(errors::CryptoError::EncodingFailed)?;
                        Ok(Self::new(value.into(), encrypted_data.clone().into_inner()))
                    }
                    None => Err(errors::CryptoError::EncodingFailed),
                },
                Err(_) => Err(errors::CryptoError::EncodingFailed),
            };
            match decrypted {
                Ok(de) => Ok(de),
                Err(_) => {
                    logger::info!("Fall back to Application Decryption");
                    Self::decrypt(encrypted_data, key, crypt_algo).await
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

    #[allow(unused_variables)]
    async fn batch_encrypt_via_api(
        state: &KeyManagerState,
        masked_data: FxHashMap<String, Secret<serde_json::Value, S>>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::batch_encrypt(masked_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(encrypted_data) => {
                    let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
                    for (k, v) in encrypted_data.data.0.iter() {
                        masked_data.get(k).map(|inner| {
                            encrypted.insert(
                                k.to_string(),
                                Self::new(inner.clone(), Secret::new(v.data.peek().clone())),
                            )
                        });
                    }
                    Ok(encrypted)
                }
                Err(_) => {
                    logger::info!("Fall back to Application Encryption");
                    Self::batch_encrypt(masked_data, key, crypt_algo).await
                }
            }
        }
    }

    #[allow(unused_variables)]
    async fn batch_decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: FxHashMap<String, Encryption>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::batch_decrypt(encrypted_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(decrypted_data) => {
                    let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
                    for (k, v) in decrypted_data.data.0.iter() {
                        let encrypted = encrypted_data
                            .get(k)
                            .ok_or(errors::CryptoError::DecodingFailed)?;
                        decrypted.insert(
                            k.to_string(),
                            Self::new(
                                serde_json::from_slice(v.clone().inner().peek().clone().to_bytes())
                                    .change_context(errors::CryptoError::DecodingFailed)?,
                                encrypted.clone().into_inner(),
                            ),
                        );
                    }
                    Ok(decrypted)
                }
                Err(_) => {
                    logger::info!("Fall back to Application Decryption");
                    Self::batch_decrypt(encrypted_data, key, crypt_algo).await
                }
            }
        }
    }

    async fn batch_encrypt(
        masked_data: FxHashMap<String, Secret<serde_json::Value, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
        for (k, v) in masked_data {
            let data =
                serde_json::to_vec(v.peek()).change_context(errors::CryptoError::DecodingFailed)?;
            encrypted.insert(
                k.clone(),
                Self::new(v, crypt_algo.encode_message(key, &data)?.into()),
            );
        }
        Ok(encrypted)
    }

    async fn batch_decrypt(
        encrypted_data: FxHashMap<String, Encryption>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
        for (k, v) in encrypted_data {
            let data = crypt_algo.decode_message(key, v.clone().into_inner().clone())?;

            let value: serde_json::Value = serde_json::from_slice(&data)
                .change_context(errors::CryptoError::DecodingFailed)?;
            decrypted.insert(k, Self::new(value.into(), v.into_inner()));
        }
        Ok(decrypted)
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
        state: &KeyManagerState,
        masked_data: Secret<Vec<u8>, S>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::encrypt(masked_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted =
                match result {
                    Ok(encrypted_data) => encrypted_data.data.0.get(DEFAULT_KEY).map(|inner| {
                        Self::new(masked_data.clone(), inner.data.peek().clone().into())
                    }),
                    Err(_) => None,
                };
            match encrypted {
                Some(en) => Ok(en),
                None => {
                    logger::info!("Fall back to Application Encryption");
                    Self::encrypt(masked_data, key, crypt_algo).await
                }
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: Encryption,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::decrypt(encrypted_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match result {
                Ok(decrypted_data) => decrypted_data.data.0.get(DEFAULT_KEY).map(|data| {
                    Self::new(
                        data.clone().inner().peek().clone().into(),
                        encrypted_data.clone().into_inner(),
                    )
                }),
                Err(_) => None,
            };
            match decrypted {
                Some(de) => Ok(de),
                None => {
                    logger::info!("Fall back to Application Decryption");
                    Self::decrypt(encrypted_data, key, crypt_algo).await
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

    #[allow(unused_variables)]
    async fn batch_encrypt_via_api(
        state: &KeyManagerState,
        masked_data: FxHashMap<String, Secret<Vec<u8>, S>>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::batch_encrypt(masked_data, key, crypt_algo).await
        }

        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(encrypted_data) => {
                    let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
                    for (k, v) in encrypted_data.data.0.iter() {
                        masked_data.get(k).map(|inner| {
                            encrypted.insert(
                                k.to_string(),
                                Self::new(inner.clone(), Secret::new(v.data.peek().clone())),
                            )
                        });
                    }
                    Ok(encrypted)
                }
                Err(_) => {
                    logger::info!("Fall back to Application Encryption");
                    Self::batch_encrypt(masked_data, key, crypt_algo).await
                }
            }
        }
    }

    #[allow(unused_variables)]
    async fn batch_decrypt_via_api(
        state: &KeyManagerState,
        encrypted_data: FxHashMap<String, Encryption>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        #[cfg(not(feature = "encryption_service"))]
        {
            Self::batch_decrypt(encrypted_data, key, crypt_algo).await
        }
        #[cfg(feature = "encryption_service")]
        {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(decrypted_data) => {
                    let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
                    for (k, v) in decrypted_data.data.0.iter() {
                        encrypted_data.get(k).map(|encrypted| {
                            decrypted.insert(
                                k.to_string(),
                                Self::new(
                                    v.clone().inner().peek().clone().into(),
                                    encrypted.clone().into_inner(),
                                ),
                            )
                        });
                    }
                    Ok(decrypted)
                }
                Err(_) => {
                    logger::info!("Fall back to Application Decryption");
                    Self::batch_decrypt(encrypted_data, key, crypt_algo).await
                }
            }
        }
    }

    async fn batch_encrypt(
        masked_data: FxHashMap<String, Secret<Vec<u8>, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
        for (k, v) in masked_data {
            encrypted.insert(
                k,
                Self::new(v.clone(), crypt_algo.encode_message(key, v.peek())?.into()),
            );
        }
        Ok(encrypted)
    }

    async fn batch_decrypt(
        encrypted_data: FxHashMap<String, Encryption>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
        for (k, v) in encrypted_data {
            decrypted.insert(
                k,
                Self::new(
                    crypt_algo
                        .decode_message(key, v.clone().into_inner().clone())?
                        .into(),
                    v.into_inner(),
                ),
            );
        }
        Ok(decrypted)
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
    state: &KeyManagerState,
    inner: Secret<E, S>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<crypto::Encryptable<Secret<E, S>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    record_operation_time(
        crypto::Encryptable::encrypt_via_api(state, inner, identifier, key, crypto::GcmAes256),
        &metrics::ENCRYPTION_TIME,
        &metrics::CONTEXT,
        &[],
    )
    .await
}

#[inline]
pub async fn batch_encrypt<E: Clone, S>(
    state: &KeyManagerState,
    inner: FxHashMap<String, Secret<E, S>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    record_operation_time(
        crypto::Encryptable::batch_encrypt_via_api(
            state,
            inner,
            identifier,
            key,
            crypto::GcmAes256,
        ),
        &metrics::ENCRYPTION_TIME,
        &metrics::CONTEXT,
        &[],
    )
    .await
}

#[inline]
pub async fn encrypt_optional<E: Clone, S>(
    state: &KeyManagerState,
    inner: Option<Secret<E, S>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    Secret<E, S>: Send,
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    inner
        .async_map(|f| encrypt(state, f, identifier, key))
        .await
        .transpose()
}

#[inline]
pub async fn batch_encrypt_optional<E: Clone, S>(
    state: &KeyManagerState,
    inner: FxHashMap<String, Option<Secret<E, S>>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    Secret<E, S>: Send,
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    let mut masked_data = FxHashMap::default();
    for (k, v) in inner {
        v.map(|masked| masked_data.insert(k, masked));
    }
    batch_encrypt(state, masked_data, identifier, key).await
}

#[inline]
pub async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    state: &KeyManagerState,
    inner: Option<Encryption>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    record_operation_time(
        inner.async_map(|item| {
            crypto::Encryptable::decrypt_via_api(state, item, identifier, key, crypto::GcmAes256)
        }),
        &metrics::DECRYPTION_TIME,
        &metrics::CONTEXT,
        &[],
    )
    .await
    .transpose()
}

#[inline]
pub async fn batch_decrypt_optional<T: Clone, S: masking::Strategy<T>>(
    state: &KeyManagerState,
    inner: FxHashMap<String, Option<Encryption>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    let mut encrypted_data = FxHashMap::default();
    for (k, v) in inner {
        v.map(|encrypted| encrypted_data.insert(k, encrypted));
    }
    batch_decrypt(state, encrypted_data, identifier, key).await
}

#[inline]
pub async fn batch_decrypt<E: Clone, S>(
    state: &KeyManagerState,
    inner: FxHashMap<String, Encryption>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    record_operation_time(
        crypto::Encryptable::batch_decrypt_via_api(
            state,
            inner,
            identifier,
            key,
            crypto::GcmAes256,
        ),
        &metrics::ENCRYPTION_TIME,
        &metrics::CONTEXT,
        &[],
    )
    .await
}

pub(crate) mod metrics {
    use router_env::{global_meter, histogram_metric, metrics_context, once_cell};

    metrics_context!(CONTEXT);
    global_meter!(GLOBAL_METER, "ROUTER_API");

    // Encryption and Decryption metrics
    histogram_metric!(ENCRYPTION_TIME, GLOBAL_METER);
    histogram_metric!(DECRYPTION_TIME, GLOBAL_METER);
}
