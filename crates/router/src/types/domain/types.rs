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
use rdkafka::message::ToBytes;
use router_env::{instrument, tracing};
use rustc_hash::FxHashMap;

#[allow(unused_imports)]
use super::{
    DecryptDataRequest, DecryptDataResponse, DecryptedData, EncryptDataRequest,
    EncryptDataResponse, EncryptedData, Identifier,
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
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<Self, errors::CryptoError>;

    async fn decrypt_via_api(
        state: &SessionState,
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
        state: &SessionState,
        masked_data: Vec<Secret<T, S>>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

    async fn batch_decrypt_via_api(
        state: &SessionState,
        encrypted_data: Vec<Encryption>,
        identifier: Identifier,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

    async fn batch_encrypt(
        masked_data: Vec<Secret<T, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

    async fn batch_decrypt(
        encrypted_data: Vec<Encryption>,
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
        state: &SessionState,
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
            let result = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match result {
                Ok(response) => match response {
                    Ok(encrypt_response) => {
                        match EncryptDataResponse::try_from(encrypt_response.response) {
                            Ok(encrypted) => {
                                encrypted.data.0.get(masked_data.clone().peek()).map(|ed| {
                                    Self::new(masked_data.clone(), ed.data.peek().clone().into())
                                })
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => Self::encrypt(masked_data, key, crypt_algo).await,
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &SessionState,
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
            let result = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match result {
                Ok(response) => match response {
                    Ok(decrypt_response) => {
                        match DecryptDataResponse::try_from(decrypt_response.response) {
                            Ok(decrypted) => {
                                let decrypted_data = decrypted.data.0.get(
                                    &String::from_utf8_lossy(
                                        encrypted_data.clone().into_inner().expose().to_bytes(),
                                    )
                                    .to_string(),
                                );
                                decrypted_data.map(|data| {
                                    Self::new(
                                        String::from_utf8_lossy(
                                            data.clone().inner().peek().to_bytes(),
                                        )
                                        .to_string()
                                        .into(),
                                        encrypted_data.clone().into_inner(),
                                    )
                                })
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match decrypted {
                Some(de) => Ok(de),
                None => Self::decrypt(encrypted_data, key, crypt_algo).await,
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

    async fn batch_encrypt_via_api(
        state: &SessionState,
        masked_data: Vec<Secret<String, S>>,
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
            let response = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match response {
                Ok(result) => match result {
                    Ok(encrypted_data) => {
                        match EncryptDataResponse::try_from(encrypted_data.response) {
                            Ok(encrypted_data_response) => {
                                let encrypted = encrypted_data_response
                                    .data
                                    .0
                                    .into_iter()
                                    .map(|(k, v)| {
                                        (
                                            k.clone(),
                                            Self::new(
                                                k.clone().into(),
                                                v.data.peek().clone().into(),
                                            ),
                                        )
                                    })
                                    .collect();
                                Some(encrypted)
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => Self::batch_encrypt(masked_data, key, crypt_algo).await,
            }
        }
    }

    async fn batch_decrypt_via_api(
        state: &SessionState,
        encrypted_data: Vec<Encryption>,
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
            let response = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match response {
                Ok(service_response) => match service_response {
                    Ok(decrypted_response) => {
                        match DecryptDataResponse::try_from(decrypted_response.response) {
                            Ok(decrypted_data) => {
                                let decrypted: FxHashMap<String, Self> = decrypted_data
                                    .data
                                    .0
                                    .into_iter()
                                    .map(|(k, v)| {
                                        (
                                            k.clone(),
                                            Self::new(k.into(), v.inner().peek().clone().into()),
                                        )
                                    })
                                    .collect();
                                Some(decrypted)
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match decrypted {
                Some(de) => Ok(de),
                None => Self::batch_decrypt(encrypted_data, key, crypt_algo).await,
            }
        }
    }

    async fn batch_encrypt(
        masked_data: Vec<Secret<String, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
        for masked in masked_data {
            let encrypted_data = crypt_algo.encode_message(key, masked.peek().as_bytes())?;
            encrypted.insert(
                masked.clone().expose(),
                Self::new(masked, encrypted_data.into()),
            );
        }
        Ok(encrypted)
    }

    async fn batch_decrypt(
        encrypted_data: Vec<Encryption>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
        for encrypted in encrypted_data {
            let encrypted_inner = encrypted.into_inner();
            let data = crypt_algo.decode_message(key, encrypted_inner.clone())?;

            let value: String = std::str::from_utf8(&data)
                .change_context(errors::CryptoError::DecodingFailed)?
                .to_string();
            decrypted.insert(
                String::from_utf8_lossy(encrypted_inner.peek()).to_string(),
                Self::new(value.into(), encrypted_inner),
            );
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
        state: &SessionState,
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
            let result = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match result {
                Ok(response) => match response {
                    Ok(encrypt_response) => {
                        match EncryptDataResponse::try_from(encrypt_response.response) {
                            Ok(encrypted) => encrypted
                                .data
                                .0
                                .get(&masked_data.clone().peek().to_string())
                                .map(|data| {
                                    Self::new(masked_data.clone(), data.data.peek().clone().into())
                                }),
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => Self::encrypt(masked_data, key, crypt_algo).await,
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &SessionState,
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
            let result = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match result {
                Ok(response) => match response {
                    Ok(decrypt_response) => {
                        match DecryptDataResponse::try_from(decrypt_response.response) {
                            Ok(decrypted) => decrypted
                                .data
                                .0
                                .get(
                                    &String::from_utf8_lossy(
                                        encrypted_data.clone().get_inner().peek(),
                                    )
                                    .to_string(),
                                )
                                .and_then(|data| {
                                    let value: Result<serde_json::Value, serde_json::Error> =
                                        serde_json::from_slice(data.clone().inner().peek());
                                    match value {
                                        Ok(val) => Some(Self::new(
                                            val.into(),
                                            encrypted_data.clone().into_inner(),
                                        )),
                                        Err(_) => None,
                                    }
                                }),
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match decrypted {
                Some(de) => Ok(de),
                None => Self::decrypt(encrypted_data, key, crypt_algo).await,
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

    async fn batch_encrypt_via_api(
        state: &SessionState,
        masked_data: Vec<Secret<serde_json::Value, S>>,
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
            let response = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match response {
                Ok(result) => match result {
                    Ok(encrypted_data) => {
                        let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
                        match EncryptDataResponse::try_from(encrypted_data.response) {
                            Ok(data) => {
                                for (k, v) in data.data.0.iter() {
                                    let masked_data = serde_json::from_str(k.as_str())
                                        .change_context(errors::CryptoError::EncodingFailed)?;
                                    let encrypted_data = Secret::new(v.data.peek().clone());
                                    encrypted.insert(
                                        k.to_string(),
                                        Self::new(masked_data, encrypted_data),
                                    );
                                }
                                Some(encrypted)
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => Self::batch_encrypt(masked_data, key, crypt_algo).await,
            }
        }
    }

    async fn batch_decrypt_via_api(
        state: &SessionState,
        encrypted_data: Vec<Encryption>,
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
            let response = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match response {
                Ok(service_response) => match service_response {
                    Ok(decrypted_response) => {
                        match DecryptDataResponse::try_from(decrypted_response.response) {
                            Ok(decrypted_data) => {
                                let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
                                for (k, v) in decrypted_data.data.0.iter() {
                                    decrypted.insert(
                                        k.to_string(),
                                        Self::new(
                                            serde_json::from_slice(
                                                v.clone().inner().peek().clone().to_bytes(),
                                            )
                                            .change_context(errors::CryptoError::DecodingFailed)?,
                                            k.as_bytes().to_vec().into(),
                                        ),
                                    );
                                }
                                Some(decrypted)
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match decrypted {
                Some(de) => Ok(de),
                None => Self::batch_decrypt(encrypted_data, key, crypt_algo).await,
            }
        }
    }

    async fn batch_encrypt(
        masked_data: Vec<Secret<serde_json::Value, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
        for masked in masked_data {
            let data = serde_json::to_vec(&masked.peek())
                .change_context(errors::CryptoError::DecodingFailed)?;
            let encrypted_data = crypt_algo.encode_message(key, &data)?;
            encrypted.insert(
                masked.clone().expose().to_string(),
                Self::new(masked, encrypted_data.into()),
            );
        }
        Ok(encrypted)
    }

    async fn batch_decrypt(
        encrypted_data: Vec<Encryption>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
        for encrypted in encrypted_data {
            let encrypted_inner = encrypted.into_inner();
            let data = crypt_algo.decode_message(key, encrypted_inner.clone())?;

            let value: serde_json::Value = serde_json::from_slice(&data)
                .change_context(errors::CryptoError::DecodingFailed)?;
            decrypted.insert(
                String::from_utf8_lossy(data.to_bytes()).to_string(),
                Self::new(value.into(), encrypted_inner),
            );
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
        state: &SessionState,
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
            let result = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match result {
                Ok(response) => match response {
                    Ok(encrypt_response) => {
                        match EncryptDataResponse::try_from(encrypt_response.response) {
                            Ok(encrypted) => encrypted
                                .data
                                .0
                                .get(&String::from_utf8_lossy(masked_data.peek()).to_string())
                                .map(|data| {
                                    Self::new(masked_data.clone(), data.data.peek().clone().into())
                                }),
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => Self::encrypt(masked_data, key, crypt_algo).await,
            }
        }
    }

    #[instrument(skip_all)]
    #[allow(unused_variables)]
    async fn decrypt_via_api(
        state: &SessionState,
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
            let result = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match result {
                Ok(response) => match response {
                    Ok(decrypt_response) => {
                        match DecryptDataResponse::try_from(decrypt_response.response) {
                            Ok(decrypted) => decrypted
                                .data
                                .0
                                .get(
                                    &String::from_utf8_lossy(
                                        encrypted_data.clone().into_inner().peek().to_bytes(),
                                    )
                                    .to_string(),
                                )
                                .map(|data| {
                                    Self::new(
                                        data.clone().inner().peek().clone().into(),
                                        encrypted_data.clone().into_inner(),
                                    )
                                }),
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match decrypted {
                Some(de) => Ok(de),
                None => Self::decrypt(encrypted_data, key, crypt_algo).await,
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

    async fn batch_encrypt_via_api(
        state: &SessionState,
        masked_data: Vec<Secret<Vec<u8>, S>>,
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
            let response = call_encryption_service(
                state,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            let encrypted = match response {
                Ok(encryption_service_result) => match encryption_service_result {
                    Ok(encryption_response) => {
                        match EncryptDataResponse::try_from(encryption_response.response) {
                            Ok(encrypted_data_response) => {
                                let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
                                for (k, v) in encrypted_data_response.data.0.iter() {
                                    let masked_data = k.as_bytes().to_vec();
                                    let encrypted_data = Secret::new(v.data.peek().clone());
                                    encrypted.insert(
                                        k.to_string(),
                                        Self::new(masked_data.into(), encrypted_data),
                                    );
                                }
                                Some(encrypted)
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match encrypted {
                Some(en) => Ok(en),
                None => Self::batch_encrypt(masked_data, key, crypt_algo).await,
            }
        }
    }

    async fn batch_decrypt_via_api(
        state: &SessionState,
        encrypted_data: Vec<Encryption>,
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
            let mut encrypted_data_group = FxHashMap::default();
            for encrypted in encrypted_data.iter() {
                let encrypted_inner = encrypted.clone().into_inner().expose();
                let k = String::from_utf8_lossy(encrypted_inner.as_slice()).to_string();
                encrypted_data_group.insert(
                    k,
                    EncryptedData {
                        data: StrongSecret::new(encrypted_inner),
                    },
                );
            }
            let response = call_encryption_service(
                state,
                "data/decrypt",
                DecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            let decrypted = match response {
                Ok(service_response) => match service_response {
                    Ok(decrypted_response) => {
                        match DecryptDataResponse::try_from(decrypted_response.response) {
                            Ok(decrypted_data) => {
                                let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
                                for (k, v) in decrypted_data.data.0.iter() {
                                    decrypted.insert(
                                        k.to_string(),
                                        Self::new(
                                            v.clone().inner().peek().clone().into(),
                                            k.as_bytes().to_vec().into(),
                                        ),
                                    );
                                }
                                Some(decrypted)
                            }
                            Err(_) => None,
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            match decrypted {
                Some(de) => Ok(de),
                None => Self::batch_decrypt(encrypted_data, key, crypt_algo).await,
            }
        }
    }

    async fn batch_encrypt(
        masked_data: Vec<Secret<Vec<u8>, S>>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut encrypted: FxHashMap<String, Self> = FxHashMap::default();
        for masked in masked_data {
            let encrypted_data = crypt_algo.encode_message(key, masked.peek())?;
            encrypted.insert(
                String::from_utf8_lossy(masked.clone().expose().as_slice()).to_string(),
                Self::new(masked, encrypted_data.into()),
            );
        }
        Ok(encrypted)
    }

    async fn batch_decrypt(
        encrypted_data: Vec<Encryption>,
        key: &[u8],
        crypt_algo: V,
    ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
        let mut decrypted: FxHashMap<String, Self> = FxHashMap::default();
        for encrypted in encrypted_data {
            let encrypted_inner = encrypted.into_inner();
            let data = crypt_algo.decode_message(key, encrypted_inner.clone())?;
            decrypted.insert(
                String::from_utf8_lossy(data.as_slice()).to_string(),
                Self::new(data.into(), encrypted_inner),
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
    state: &SessionState,
    inner: Secret<E, S>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<crypto::Encryptable<Secret<E, S>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        crypto::Encryptable::encrypt_via_api(state, inner, identifier, key, crypto::GcmAes256),
        &ENCRYPTION_TIME,
        &[],
    )
    .await
}

#[inline]
pub async fn batch_encrypt<E: Clone, S>(
    state: &SessionState,
    inner: Vec<Secret<E, S>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        crypto::Encryptable::batch_encrypt_via_api(
            state,
            inner,
            identifier,
            key,
            crypto::GcmAes256,
        ),
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
    state: &SessionState,
    inner: Vec<Option<Secret<E, S>>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    Secret<E, S>: Send,
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    let mut masked_data: Vec<Secret<E, S>> = vec![];
    for item in inner {
        item.map(|masked| masked_data.push(masked));
    }
    batch_encrypt(state, masked_data, identifier, key).await
}

#[inline]
pub async fn decrypt<T: Clone, S: masking::Strategy<T>>(
    state: &SessionState,
    inner: Option<Encryption>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<Option<crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        inner.async_map(|item| {
            crypto::Encryptable::decrypt_via_api(state, item, identifier, key, crypto::GcmAes256)
        }),
        &DECRYPTION_TIME,
        &[],
    )
    .await
    .transpose()
}

#[inline]
pub async fn batch_decrypt_optional<T: Clone, S: masking::Strategy<T>>(
    state: &SessionState,
    inner: Vec<Option<Encryption>>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<T, S>>>, errors::CryptoError>
where
    crypto::Encryptable<Secret<T, S>>: TypeEncryption<T, crypto::GcmAes256, S>,
{
    let mut encrypted_data = vec![];
    for item in inner {
        item.map(|encrypted| encrypted_data.push(encrypted));
    }
    batch_decrypt(state, encrypted_data, identifier, key).await
}

#[inline]
pub async fn batch_decrypt<E: Clone, S>(
    state: &SessionState,
    inner: Vec<Encryption>,
    identifier: Identifier,
    key: &[u8],
) -> CustomResult<FxHashMap<String, crypto::Encryptable<Secret<E, S>>>, errors::CryptoError>
where
    S: masking::Strategy<E>,
    crypto::Encryptable<Secret<E, S>>: TypeEncryption<E, crypto::GcmAes256, S>,
{
    request::record_operation_time(
        crypto::Encryptable::batch_decrypt_via_api(
            state,
            inner,
            identifier,
            key,
            crypto::GcmAes256,
        ),
        &ENCRYPTION_TIME,
        &[],
    )
    .await
}
