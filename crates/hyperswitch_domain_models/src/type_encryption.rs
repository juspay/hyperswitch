use async_trait::async_trait;
use common_utils::{
    crypto,
    encryption::Encryption,
    errors::{self, CustomResult},
    ext_traits::AsyncExt,
    metrics::utils::record_operation_time,
    types::keymanager::{Identifier, KeyManagerState},
};
use encrypt::TypeEncryption;
use masking::Secret;
use rustc_hash::FxHashMap;

mod encrypt {
    use async_trait::async_trait;
    use common_utils::{
        crypto,
        encryption::Encryption,
        errors::{self, CustomResult},
        keymanager::call_encryption_service,
        transformers::{ForeignFrom, ForeignTryFrom},
        types::keymanager::{
            BatchDecryptDataResponse, BatchEncryptDataRequest, BatchEncryptDataResponse,
            DecryptDataResponse, EncryptDataRequest, EncryptDataResponse, Identifier,
            KeyManagerState, TransientBatchDecryptDataRequest, TransientDecryptDataRequest,
        },
    };
    use error_stack::ResultExt;
    use http::Method;
    use masking::{PeekInterface, Secret};
    use router_env::{instrument, logger, tracing};
    use rustc_hash::FxHashMap;

    use super::metrics;

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

    fn is_encryption_service_enabled(_state: &KeyManagerState) -> bool {
        #[cfg(feature = "encryption_service")]
        {
            _state.enabled.unwrap_or_default()
        }
        #[cfg(not(feature = "encryption_service"))]
        {
            false
        }
    }

    #[async_trait]
    impl<
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<String> + Send + Sync,
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::encrypt(masked_data, key, crypt_algo).await
            } else {
                let result: Result<
                    EncryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/encrypt",
                    EncryptDataRequest::from((masked_data.clone(), identifier)),
                )
                .await;
                match result {
                    Ok(response) => Ok(ForeignFrom::foreign_from((masked_data.clone(), response))),
                    Err(err) => {
                        logger::error!("Encryption error {:?}", err);
                        metrics::ENCRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::decrypt(encrypted_data, key, crypt_algo).await
            } else {
                let result: Result<
                    DecryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/decrypt",
                    TransientDecryptDataRequest::from((encrypted_data.clone(), identifier)),
                )
                .await;
                let decrypted = match result {
                    Ok(decrypted_data) => {
                        ForeignTryFrom::foreign_try_from((encrypted_data.clone(), decrypted_data))
                    }
                    Err(err) => {
                        logger::error!("Decryption error {:?}", err);
                        Err(err.change_context(errors::CryptoError::DecodingFailed))
                    }
                };

                match decrypted {
                    Ok(de) => Ok(de),
                    Err(_) => {
                        metrics::DECRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            metrics::APPLICATION_ENCRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;
            Ok(Self::new(masked_data, encrypted_data.into()))
        }

        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::batch_encrypt(masked_data, key, crypt_algo).await
            } else {
                let result: Result<
                    BatchEncryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/encrypt",
                    BatchEncryptDataRequest::from((masked_data.clone(), identifier)),
                )
                .await;
                match result {
                    Ok(response) => Ok(ForeignFrom::foreign_from((masked_data, response))),
                    Err(err) => {
                        metrics::ENCRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                        logger::error!("Encryption error {:?}", err);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::batch_decrypt(encrypted_data, key, crypt_algo).await
            } else {
                let result: Result<
                    BatchDecryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/decrypt",
                    TransientBatchDecryptDataRequest::from((encrypted_data.clone(), identifier)),
                )
                .await;
                let decrypted = match result {
                    Ok(decrypted_data) => {
                        ForeignTryFrom::foreign_try_from((encrypted_data.clone(), decrypted_data))
                    }
                    Err(err) => {
                        logger::error!("Decryption error {:?}", err);
                        Err(err.change_context(errors::CryptoError::DecodingFailed))
                    }
                };
                match decrypted {
                    Ok(de) => Ok(de),
                    Err(_) => {
                        metrics::DECRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            metrics::APPLICATION_ENCRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            masked_data
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k,
                        Self::new(
                            v.clone(),
                            crypt_algo.encode_message(key, v.peek().as_bytes())?.into(),
                        ),
                    ))
                })
                .collect()
        }

        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            encrypted_data
                .into_iter()
                .map(|(k, v)| {
                    let data = crypt_algo.decode_message(key, v.clone().into_inner())?;
                    let value: String = std::str::from_utf8(&data)
                        .change_context(errors::CryptoError::DecodingFailed)?
                        .to_string();
                    Ok((k, Self::new(value.into(), v.into_inner())))
                })
                .collect()
        }
    }

    #[async_trait]
    impl<
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<serde_json::Value> + Send + Sync,
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::encrypt(masked_data, key, crypt_algo).await
            } else {
                let result: Result<
                    EncryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/encrypt",
                    EncryptDataRequest::from((masked_data.clone(), identifier)),
                )
                .await;
                match result {
                    Ok(response) => Ok(ForeignFrom::foreign_from((masked_data.clone(), response))),
                    Err(err) => {
                        logger::error!("Encryption error {:?}", err);
                        metrics::ENCRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::decrypt(encrypted_data, key, crypt_algo).await
            } else {
                let result: Result<
                    DecryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/decrypt",
                    TransientDecryptDataRequest::from((encrypted_data.clone(), identifier)),
                )
                .await;
                let decrypted = match result {
                    Ok(decrypted_data) => {
                        ForeignTryFrom::foreign_try_from((encrypted_data.clone(), decrypted_data))
                    }
                    Err(err) => {
                        logger::error!("Decryption error {:?}", err);
                        Err(err.change_context(errors::CryptoError::EncodingFailed))
                    }
                };
                match decrypted {
                    Ok(de) => Ok(de),
                    Err(_) => {
                        metrics::DECRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            metrics::APPLICATION_ENCRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
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
            metrics::APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            let encrypted = encrypted_data.into_inner();
            let data = crypt_algo.decode_message(key, encrypted.clone())?;

            let value: serde_json::Value = serde_json::from_slice(&data)
                .change_context(errors::CryptoError::DecodingFailed)?;
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::batch_encrypt(masked_data, key, crypt_algo).await
            } else {
                let result: Result<
                    BatchEncryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/encrypt",
                    BatchEncryptDataRequest::from((masked_data.clone(), identifier)),
                )
                .await;
                match result {
                    Ok(response) => Ok(ForeignFrom::foreign_from((masked_data, response))),
                    Err(err) => {
                        metrics::ENCRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                        logger::error!("Encryption error {:?}", err);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::batch_decrypt(encrypted_data, key, crypt_algo).await
            } else {
                let result: Result<
                    BatchDecryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/decrypt",
                    TransientBatchDecryptDataRequest::from((encrypted_data.clone(), identifier)),
                )
                .await;
                let decrypted = match result {
                    Ok(decrypted_data) => {
                        ForeignTryFrom::foreign_try_from((encrypted_data.clone(), decrypted_data))
                    }
                    Err(err) => {
                        logger::error!("Decryption error {:?}", err);
                        Err(err.change_context(errors::CryptoError::DecodingFailed))
                    }
                };
                match decrypted {
                    Ok(de) => Ok(de),
                    Err(_) => {
                        metrics::DECRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            metrics::APPLICATION_ENCRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            masked_data
                .into_iter()
                .map(|(k, v)| {
                    let data = serde_json::to_vec(v.peek())
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    Ok((
                        k,
                        Self::new(v, crypt_algo.encode_message(key, &data)?.into()),
                    ))
                })
                .collect()
        }

        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            encrypted_data
                .into_iter()
                .map(|(k, v)| {
                    let data = crypt_algo.decode_message(key, v.clone().into_inner().clone())?;

                    let value: serde_json::Value = serde_json::from_slice(&data)
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    Ok((k, Self::new(value.into(), v.into_inner())))
                })
                .collect()
        }
    }

    #[async_trait]
    impl<
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<Vec<u8>> + Send + Sync,
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::encrypt(masked_data, key, crypt_algo).await
            } else {
                let result: Result<
                    EncryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/encrypt",
                    EncryptDataRequest::from((masked_data.clone(), identifier)),
                )
                .await;
                match result {
                    Ok(response) => Ok(ForeignFrom::foreign_from((masked_data.clone(), response))),
                    Err(err) => {
                        logger::error!("Encryption error {:?}", err);
                        metrics::ENCRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::decrypt(encrypted_data, key, crypt_algo).await
            } else {
                let result: Result<
                    DecryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/decrypt",
                    TransientDecryptDataRequest::from((encrypted_data.clone(), identifier)),
                )
                .await;
                let decrypted = match result {
                    Ok(decrypted_data) => {
                        ForeignTryFrom::foreign_try_from((encrypted_data.clone(), decrypted_data))
                    }
                    Err(err) => {
                        logger::error!("Decryption error {:?}", err);
                        Err(err.change_context(errors::CryptoError::DecodingFailed))
                    }
                };
                match decrypted {
                    Ok(de) => Ok(de),
                    Err(_) => {
                        metrics::DECRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            metrics::APPLICATION_ENCRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            let encrypted_data = crypt_algo.encode_message(key, masked_data.peek())?;
            Ok(Self::new(masked_data, encrypted_data.into()))
        }

        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::batch_encrypt(masked_data, key, crypt_algo).await
            } else {
                let result: Result<
                    BatchEncryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/encrypt",
                    BatchEncryptDataRequest::from((masked_data.clone(), identifier)),
                )
                .await;
                match result {
                    Ok(response) => Ok(ForeignFrom::foreign_from((masked_data, response))),
                    Err(err) => {
                        metrics::ENCRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
                        logger::error!("Encryption error {:?}", err);
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
            // If encryption service is not enabled, fall back to application encryption or else call encryption service
            if !is_encryption_service_enabled(state) {
                Self::batch_decrypt(encrypted_data, key, crypt_algo).await
            } else {
                let result: Result<
                    BatchDecryptDataResponse,
                    error_stack::Report<errors::KeyManagerClientError>,
                > = call_encryption_service(
                    state,
                    Method::POST,
                    "data/decrypt",
                    TransientBatchDecryptDataRequest::from((encrypted_data.clone(), identifier)),
                )
                .await;
                let decrypted = match result {
                    Ok(response) => {
                        ForeignTryFrom::foreign_try_from((encrypted_data.clone(), response))
                    }
                    Err(err) => {
                        logger::error!("Decryption error {:?}", err);
                        Err(err.change_context(errors::CryptoError::DecodingFailed))
                    }
                };
                match decrypted {
                    Ok(de) => Ok(de),
                    Err(_) => {
                        metrics::DECRYPTION_API_FAILURES.add(&metrics::CONTEXT, 1, &[]);
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
            metrics::APPLICATION_ENCRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            masked_data
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k,
                        Self::new(v.clone(), crypt_algo.encode_message(key, v.peek())?.into()),
                    ))
                })
                .collect()
        }

        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(&metrics::CONTEXT, 1, &[]);
            encrypted_data
                .into_iter()
                .map(|(k, v)| {
                    Ok((
                        k,
                        Self::new(
                            crypt_algo
                                .decode_message(key, v.clone().into_inner().clone())?
                                .into(),
                            v.into_inner(),
                        ),
                    ))
                })
                .collect()
        }
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
    if !inner.is_empty() {
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
    } else {
        Ok(FxHashMap::default())
    }
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
pub async fn decrypt_optional<T: Clone, S: masking::Strategy<T>>(
    state: &KeyManagerState,
    inner: Option<Encryption>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    inner
        .async_map(|item| decrypt(state, item, identifier, key))
        .await
        .transpose()
}

#[inline]
pub async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    state: &KeyManagerState,
    inner: Encryption,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<crypto::Encryptable<Secret<T, S>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    record_operation_time(
        crypto::Encryptable::decrypt_via_api(state, inner, identifier, key, crypto::GcmAes256),
        &metrics::DECRYPTION_TIME,
        &metrics::CONTEXT,
        &[],
    )
    .await
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
    if !inner.is_empty() {
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
    } else {
        Ok(FxHashMap::default())
    }
}

pub(crate) mod metrics {
    use router_env::{counter_metric, global_meter, histogram_metric, metrics_context, once_cell};

    metrics_context!(CONTEXT);
    global_meter!(GLOBAL_METER, "ROUTER_API");

    // Encryption and Decryption metrics
    histogram_metric!(ENCRYPTION_TIME, GLOBAL_METER);
    histogram_metric!(DECRYPTION_TIME, GLOBAL_METER);
    counter_metric!(ENCRYPTION_API_FAILURES, GLOBAL_METER);
    counter_metric!(DECRYPTION_API_FAILURES, GLOBAL_METER);
    counter_metric!(APPLICATION_ENCRYPTION_COUNT, GLOBAL_METER);
    counter_metric!(APPLICATION_DECRYPTION_COUNT, GLOBAL_METER);
}
