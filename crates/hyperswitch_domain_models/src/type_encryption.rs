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
use router_env::{instrument, tracing};
use rustc_hash::FxHashMap;

mod encrypt {
    use async_trait::async_trait;
    use common_utils::{
        crypto,
        encryption::Encryption,
        errors::{self, CustomResult},
        ext_traits::ByteSliceExt,
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

    use super::{metrics, EncryptedJsonType};

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
            _state.enabled
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
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::ENCRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Encryption");
                        Self::encrypt(masked_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::DECRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Decryption");
                        Self::decrypt(encrypted_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<String, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_ENCRYPTION_COUNT.add(1, &[]);
            let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;
            Ok(Self::new(masked_data, encrypted_data.into()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(1, &[]);
            let encrypted = encrypted_data.into_inner();
            let data = crypt_algo.decode_message(key, encrypted.clone())?;

            let value: String = std::str::from_utf8(&data)
                .change_context(errors::CryptoError::DecodingFailed)?
                .to_string();

            Ok(Self::new(value.into(), encrypted))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::ENCRYPTION_API_FAILURES.add(1, &[]);
                        logger::error!("Encryption error {:?}", err);
                        logger::info!("Fall back to Application Encryption");
                        Self::batch_encrypt(masked_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::DECRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Decryption");
                        Self::batch_decrypt(encrypted_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<String, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_ENCRYPTION_COUNT.add(1, &[]);
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

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(1, &[]);
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
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::ENCRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Encryption");
                        Self::encrypt(masked_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::DECRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Decryption");
                        Self::decrypt(encrypted_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<serde_json::Value, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_ENCRYPTION_COUNT.add(1, &[]);
            let data = serde_json::to_vec(&masked_data.peek())
                .change_context(errors::CryptoError::DecodingFailed)?;
            let encrypted_data = crypt_algo.encode_message(key, &data)?;
            Ok(Self::new(masked_data, encrypted_data.into()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(1, &[]);
            let encrypted = encrypted_data.into_inner();
            let data = crypt_algo.decode_message(key, encrypted.clone())?;

            let value: serde_json::Value = serde_json::from_slice(&data)
                .change_context(errors::CryptoError::DecodingFailed)?;
            Ok(Self::new(value.into(), encrypted))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::ENCRYPTION_API_FAILURES.add(1, &[]);
                        logger::error!("Encryption error {:?}", err);
                        logger::info!("Fall back to Application Encryption");
                        Self::batch_encrypt(masked_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::DECRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Decryption");
                        Self::batch_decrypt(encrypted_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<serde_json::Value, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_ENCRYPTION_COUNT.add(1, &[]);
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

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(1, &[]);
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

    impl<T> EncryptedJsonType<T>
    where
        T: std::fmt::Debug + Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        fn serialize_json_bytes(&self) -> CustomResult<Secret<Vec<u8>>, errors::CryptoError> {
            common_utils::ext_traits::Encode::encode_to_vec(self.inner())
                .change_context(errors::CryptoError::EncodingFailed)
                .attach_printable("Failed to JSON serialize data before encryption")
                .map(Secret::new)
        }

        fn deserialize_json_bytes<S>(
            bytes: Secret<Vec<u8>>,
        ) -> CustomResult<Secret<Self, S>, errors::ParsingError>
        where
            S: masking::Strategy<Self>,
        {
            bytes
                .peek()
                .as_slice()
                .parse_struct::<T>(std::any::type_name::<T>())
                .map(|result| Secret::new(Self::from(result)))
                .attach_printable("Failed to JSON deserialize data after decryption")
        }
    }

    #[async_trait]
    impl<
            T: std::fmt::Debug + Clone + serde::Serialize + serde::de::DeserializeOwned + Send,
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<EncryptedJsonType<T>> + Send + Sync,
        > TypeEncryption<EncryptedJsonType<T>, V, S>
        for crypto::Encryptable<Secret<EncryptedJsonType<T>, S>>
    {
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt_via_api(
            state: &KeyManagerState,
            masked_data: Secret<EncryptedJsonType<T>, S>,
            identifier: Identifier,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let data_bytes = EncryptedJsonType::serialize_json_bytes(masked_data.peek())?;
            let result: crypto::Encryptable<Secret<Vec<u8>>> =
                TypeEncryption::encrypt_via_api(state, data_bytes, identifier, key, crypt_algo)
                    .await?;
            Ok(Self::new(masked_data, result.into_encrypted()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt_via_api(
            state: &KeyManagerState,
            encrypted_data: Encryption,
            identifier: Identifier,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: crypto::Encryptable<Secret<Vec<u8>>> =
                TypeEncryption::decrypt_via_api(state, encrypted_data, identifier, key, crypt_algo)
                    .await?;
            result
                .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                .change_context(errors::CryptoError::DecodingFailed)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<EncryptedJsonType<T>, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let data_bytes = EncryptedJsonType::serialize_json_bytes(masked_data.peek())?;
            let result: crypto::Encryptable<Secret<Vec<u8>>> =
                TypeEncryption::encrypt(data_bytes, key, crypt_algo).await?;
            Ok(Self::new(masked_data, result.into_encrypted()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: crypto::Encryptable<Secret<Vec<u8>>> =
                TypeEncryption::decrypt(encrypted_data, key, crypt_algo).await?;
            result
                .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                .change_context(errors::CryptoError::DecodingFailed)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt_via_api(
            state: &KeyManagerState,
            masked_data: FxHashMap<String, Secret<EncryptedJsonType<T>, S>>,
            identifier: Identifier,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let hashmap_capacity = masked_data.len();
            let data_bytes = masked_data.iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let value_bytes = EncryptedJsonType::serialize_json_bytes(value.peek())?;
                    map.insert(key.to_owned(), value_bytes);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>>>> =
                TypeEncryption::batch_encrypt_via_api(
                    state, data_bytes, identifier, key, crypt_algo,
                )
                .await?;
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let original_value = masked_data
                        .get(&key)
                        .ok_or(errors::CryptoError::EncodingFailed)
                        .attach_printable_lazy(|| {
                            format!("Failed to find {key} in input hashmap")
                        })?;
                    map.insert(
                        key,
                        Self::new(original_value.clone(), value.into_encrypted()),
                    );
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt_via_api(
            state: &KeyManagerState,
            encrypted_data: FxHashMap<String, Encryption>,
            identifier: Identifier,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>>>> =
                TypeEncryption::batch_decrypt_via_api(
                    state,
                    encrypted_data,
                    identifier,
                    key,
                    crypt_algo,
                )
                .await?;

            let hashmap_capacity = result.len();
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let deserialized_value = value
                        .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    map.insert(key, deserialized_value);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<EncryptedJsonType<T>, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let hashmap_capacity = masked_data.len();
            let data_bytes = masked_data.iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let value_bytes = EncryptedJsonType::serialize_json_bytes(value.peek())?;
                    map.insert(key.to_owned(), value_bytes);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>>>> =
                TypeEncryption::batch_encrypt(data_bytes, key, crypt_algo).await?;
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let original_value = masked_data
                        .get(&key)
                        .ok_or(errors::CryptoError::EncodingFailed)
                        .attach_printable_lazy(|| {
                            format!("Failed to find {key} in input hashmap")
                        })?;
                    map.insert(
                        key,
                        Self::new(original_value.clone(), value.into_encrypted()),
                    );
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>>>> =
                TypeEncryption::batch_decrypt(encrypted_data, key, crypt_algo).await?;

            let hashmap_capacity = result.len();
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let deserialized_value = value
                        .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    map.insert(key, deserialized_value);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }
    }

    #[async_trait]
    impl<
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<Vec<u8>> + Send + Sync,
        > TypeEncryption<Vec<u8>, V, S> for crypto::Encryptable<Secret<Vec<u8>, S>>
    {
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::ENCRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Encryption");
                        Self::encrypt(masked_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::DECRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Decryption");
                        Self::decrypt(encrypted_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<Vec<u8>, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_ENCRYPTION_COUNT.add(1, &[]);
            let encrypted_data = crypt_algo.encode_message(key, masked_data.peek())?;
            Ok(Self::new(masked_data, encrypted_data.into()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(1, &[]);
            let encrypted = encrypted_data.into_inner();
            let data = crypt_algo.decode_message(key, encrypted.clone())?;
            Ok(Self::new(data.into(), encrypted))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::ENCRYPTION_API_FAILURES.add(1, &[]);
                        logger::error!("Encryption error {:?}", err);
                        logger::info!("Fall back to Application Encryption");
                        Self::batch_encrypt(masked_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
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
                        metrics::DECRYPTION_API_FAILURES.add(1, &[]);
                        logger::info!("Fall back to Application Decryption");
                        Self::batch_decrypt(encrypted_data, key, crypt_algo).await
                    }
                }
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<Vec<u8>, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_ENCRYPTION_COUNT.add(1, &[]);
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

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            metrics::APPLICATION_DECRYPTION_COUNT.add(1, &[]);
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedJsonType<T>(T);

impl<T> EncryptedJsonType<T> {
    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for EncryptedJsonType<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> std::ops::Deref for EncryptedJsonType<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

/// Type alias for `Option<Encryptable<Secret<EncryptedJsonType<T>>>>`
pub type OptionalEncryptableJsonType<T> = Option<crypto::Encryptable<Secret<EncryptedJsonType<T>>>>;

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
async fn encrypt<E: Clone, S>(
    state: &KeyManagerState,
    inner: Secret<E, S>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<crypto::Encryptable<Secret<E, S>>, CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    record_operation_time(
        crypto::Encryptable::encrypt_via_api(state, inner, identifier, key, crypto::GcmAes256),
        &metrics::ENCRYPTION_TIME,
        &[],
    )
    .await
}

#[inline]
async fn batch_encrypt<E: Clone, S>(
    state: &KeyManagerState,
    inner: FxHashMap<String, Secret<E, S>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, CryptoError>
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
            &[],
        )
        .await
    } else {
        Ok(FxHashMap::default())
    }
}

#[inline]
async fn encrypt_optional<E: Clone, S>(
    state: &KeyManagerState,
    inner: Option<Secret<E, S>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<E, S>>>, CryptoError>
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
async fn decrypt_optional<T: Clone, S: masking::Strategy<T>>(
    state: &KeyManagerState,
    inner: Option<Encryption>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    inner
        .async_map(|item| decrypt(state, item, identifier, key))
        .await
        .transpose()
}

#[inline]
async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    state: &KeyManagerState,
    inner: Encryption,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<crypto::Encryptable<Secret<T, S>>, CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    record_operation_time(
        crypto::Encryptable::decrypt_via_api(state, inner, identifier, key, crypto::GcmAes256),
        &metrics::DECRYPTION_TIME,
        &[],
    )
    .await
}

#[inline]
async fn batch_decrypt<E: Clone, S>(
    state: &KeyManagerState,
    inner: FxHashMap<String, Encryption>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, CryptoError>
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
            &[],
        )
        .await
    } else {
        Ok(FxHashMap::default())
    }
}

pub enum CryptoOperation<T: Clone, S: masking::Strategy<T>> {
    Encrypt(Secret<T, S>),
    EncryptOptional(Option<Secret<T, S>>),
    Decrypt(Encryption),
    DecryptOptional(Option<Encryption>),
    BatchEncrypt(FxHashMap<String, Secret<T, S>>),
    BatchDecrypt(FxHashMap<String, Encryption>),
}

use errors::CryptoError;

#[derive(router_derive::TryGetEnumVariant)]
#[error(CryptoError::EncodingFailed)]
pub enum CryptoOutput<T: Clone, S: masking::Strategy<T>> {
    Operation(crypto::Encryptable<Secret<T, S>>),
    OptionalOperation(Option<crypto::Encryptable<Secret<T, S>>>),
    BatchOperation(FxHashMap<String, crypto::Encryptable<Secret<T, S>>>),
}

// Do not remove the `skip_all` as the key would be logged otherwise
#[instrument(skip_all, fields(table = table_name))]
pub async fn crypto_operation<T: Clone + Send, S: masking::Strategy<T>>(
    state: &KeyManagerState,
    table_name: &str,
    operation: CryptoOperation<T, S>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<CryptoOutput<T, S>, CryptoError>
where
    Secret<T, S>: Send,
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    match operation {
        CryptoOperation::Encrypt(data) => {
            let data = encrypt(state, data, identifier, key).await?;
            Ok(CryptoOutput::Operation(data))
        }
        CryptoOperation::EncryptOptional(data) => {
            let data = encrypt_optional(state, data, identifier, key).await?;
            Ok(CryptoOutput::OptionalOperation(data))
        }
        CryptoOperation::Decrypt(data) => {
            let data = decrypt(state, data, identifier, key).await?;
            Ok(CryptoOutput::Operation(data))
        }
        CryptoOperation::DecryptOptional(data) => {
            let data = decrypt_optional(state, data, identifier, key).await?;
            Ok(CryptoOutput::OptionalOperation(data))
        }
        CryptoOperation::BatchEncrypt(data) => {
            let data = batch_encrypt(state, data, identifier, key).await?;
            Ok(CryptoOutput::BatchOperation(data))
        }
        CryptoOperation::BatchDecrypt(data) => {
            let data = batch_decrypt(state, data, identifier, key).await?;
            Ok(CryptoOutput::BatchOperation(data))
        }
    }
}

pub(crate) mod metrics {
    use router_env::{counter_metric, global_meter, histogram_metric_f64};

    global_meter!(GLOBAL_METER, "ROUTER_API");

    // Encryption and Decryption metrics
    histogram_metric_f64!(ENCRYPTION_TIME, GLOBAL_METER);
    histogram_metric_f64!(DECRYPTION_TIME, GLOBAL_METER);
    counter_metric!(ENCRYPTION_API_FAILURES, GLOBAL_METER);
    counter_metric!(DECRYPTION_API_FAILURES, GLOBAL_METER);
    counter_metric!(APPLICATION_ENCRYPTION_COUNT, GLOBAL_METER);
    counter_metric!(APPLICATION_DECRYPTION_COUNT, GLOBAL_METER);
}
