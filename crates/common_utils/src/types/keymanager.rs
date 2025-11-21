#![allow(missing_docs)]

use core::fmt;

use base64::Engine;
use masking::{ExposeInterface, PeekInterface, Secret, Strategy, StrongSecret};
#[cfg(feature = "encryption_service")]
use router_env::logger;
#[cfg(feature = "km_forward_x_request_id")]
use router_env::RequestId;
use rustc_hash::FxHashMap;
use serde::{
    de::{self, Unexpected, Visitor},
    ser, Deserialize, Deserializer, Serialize,
};

use crate::{
    consts::BASE64_ENGINE,
    crypto::Encryptable,
    encryption::Encryption,
    errors::{self, CustomResult},
    id_type,
    transformers::{ForeignFrom, ForeignTryFrom},
};

macro_rules! impl_get_tenant_for_request {
    ($ty:ident) => {
        impl GetKeymanagerTenant for $ty {
            fn get_tenant_id(&self, state: &KeyManagerState) -> id_type::TenantId {
                match self.identifier {
                    Identifier::User(_) | Identifier::UserAuth(_) => state.global_tenant_id.clone(),
                    Identifier::Merchant(_) => state.tenant_id.clone(),
                }
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct KeyManagerState {
    pub tenant_id: id_type::TenantId,
    pub global_tenant_id: id_type::TenantId,
    pub enabled: bool,
    pub url: String,
    pub client_idle_timeout: Option<u64>,
    #[cfg(feature = "km_forward_x_request_id")]
    pub request_id: Option<RequestId>,
    #[cfg(feature = "keymanager_mtls")]
    pub ca: Secret<String>,
    #[cfg(feature = "keymanager_mtls")]
    pub cert: Secret<String>,
    pub infra_values: Option<serde_json::Value>,
}

impl Default for KeyManagerState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyManagerState {
    pub fn new() -> Self {
        Self {
            tenant_id: id_type::TenantId::get_default_tenant_id(),
            global_tenant_id: id_type::TenantId::get_default_global_tenant_id(),
            enabled: Default::default(),
            url: String::default(),
            client_idle_timeout: Default::default(),
            #[cfg(feature = "km_forward_x_request_id")]
            request_id: Default::default(),
            #[cfg(feature = "keymanager_mtls")]
            ca: Default::default(),
            #[cfg(feature = "keymanager_mtls")]
            cert: Default::default(),
            infra_values: Default::default(),
        }
    }
    pub fn add_confirm_value_in_infra_values(
        &self,
        is_confirm_operation: bool,
    ) -> Option<serde_json::Value> {
        self.infra_values.clone().map(|mut infra_values| {
            if is_confirm_operation {
                infra_values.as_object_mut().map(|obj| {
                    obj.insert(
                        "is_confirm_operation".to_string(),
                        serde_json::Value::Bool(true),
                    )
                });
            }
            infra_values
        })
    }
}

pub trait GetKeymanagerTenant {
    fn get_tenant_id(&self, state: &KeyManagerState) -> id_type::TenantId;
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(tag = "data_identifier", content = "key_identifier")]
pub enum Identifier {
    User(String),
    Merchant(id_type::MerchantId),
    UserAuth(String),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct EncryptionCreateRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct EncryptionTransferRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataKeyCreateResponse {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BatchEncryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptedDataGroup,
}

impl_get_tenant_for_request!(EncryptionCreateRequest);
impl_get_tenant_for_request!(EncryptionTransferRequest);
impl_get_tenant_for_request!(BatchEncryptDataRequest);

impl<S> From<(Secret<Vec<u8>, S>, Identifier)> for EncryptDataRequest
where
    S: Strategy<Vec<u8>>,
{
    fn from((secret, identifier): (Secret<Vec<u8>, S>, Identifier)) -> Self {
        Self {
            identifier,
            data: DecryptedData(StrongSecret::new(secret.expose())),
        }
    }
}

impl<S> From<(FxHashMap<String, Secret<Vec<u8>, S>>, Identifier)> for BatchEncryptDataRequest
where
    S: Strategy<Vec<u8>>,
{
    fn from((map, identifier): (FxHashMap<String, Secret<Vec<u8>, S>>, Identifier)) -> Self {
        let group = map
            .into_iter()
            .map(|(key, value)| (key, DecryptedData(StrongSecret::new(value.expose()))))
            .collect();
        Self {
            identifier,
            data: DecryptedDataGroup(group),
        }
    }
}

impl<S> From<(Secret<String, S>, Identifier)> for EncryptDataRequest
where
    S: Strategy<String>,
{
    fn from((secret, identifier): (Secret<String, S>, Identifier)) -> Self {
        Self {
            data: DecryptedData(StrongSecret::new(secret.expose().as_bytes().to_vec())),
            identifier,
        }
    }
}

impl<S> From<(Secret<serde_json::Value, S>, Identifier)> for EncryptDataRequest
where
    S: Strategy<serde_json::Value>,
{
    fn from((secret, identifier): (Secret<serde_json::Value, S>, Identifier)) -> Self {
        Self {
            data: DecryptedData(StrongSecret::new(
                secret.expose().to_string().as_bytes().to_vec(),
            )),
            identifier,
        }
    }
}

impl<S> From<(FxHashMap<String, Secret<serde_json::Value, S>>, Identifier)>
    for BatchEncryptDataRequest
where
    S: Strategy<serde_json::Value>,
{
    fn from(
        (map, identifier): (FxHashMap<String, Secret<serde_json::Value, S>>, Identifier),
    ) -> Self {
        let group = map
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    DecryptedData(StrongSecret::new(
                        value.expose().to_string().as_bytes().to_vec(),
                    )),
                )
            })
            .collect();
        Self {
            data: DecryptedDataGroup(group),
            identifier,
        }
    }
}

impl<S> From<(FxHashMap<String, Secret<String, S>>, Identifier)> for BatchEncryptDataRequest
where
    S: Strategy<String>,
{
    fn from((map, identifier): (FxHashMap<String, Secret<String, S>>, Identifier)) -> Self {
        let group = map
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    DecryptedData(StrongSecret::new(value.expose().as_bytes().to_vec())),
                )
            })
            .collect();
        Self {
            data: DecryptedDataGroup(group),
            identifier,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptedData,
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct DecryptedDataGroup(pub FxHashMap<String, DecryptedData>);

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchEncryptDataResponse {
    pub data: EncryptedDataGroup,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptDataResponse {
    pub data: EncryptedData,
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct EncryptedDataGroup(pub FxHashMap<String, EncryptedData>);
#[derive(Debug)]
pub struct TransientBatchDecryptDataRequest {
    pub identifier: Identifier,
    pub data: FxHashMap<String, StrongSecret<Vec<u8>>>,
}

#[derive(Debug)]
pub struct TransientDecryptDataRequest {
    pub identifier: Identifier,
    pub data: StrongSecret<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchDecryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: FxHashMap<String, StrongSecret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: StrongSecret<String>,
}

impl_get_tenant_for_request!(EncryptDataRequest);
impl_get_tenant_for_request!(TransientBatchDecryptDataRequest);
impl_get_tenant_for_request!(TransientDecryptDataRequest);
impl_get_tenant_for_request!(BatchDecryptDataRequest);
impl_get_tenant_for_request!(DecryptDataRequest);

impl<T, S> ForeignFrom<(FxHashMap<String, Secret<T, S>>, BatchEncryptDataResponse)>
    for FxHashMap<String, Encryptable<Secret<T, S>>>
where
    T: Clone,
    S: Strategy<T> + Send,
{
    fn foreign_from(
        (mut masked_data, response): (FxHashMap<String, Secret<T, S>>, BatchEncryptDataResponse),
    ) -> Self {
        response
            .data
            .0
            .into_iter()
            .flat_map(|(k, v)| {
                masked_data.remove(&k).map(|inner| {
                    (
                        k,
                        Encryptable::new(inner.clone(), v.data.peek().clone().into()),
                    )
                })
            })
            .collect()
    }
}

impl<T, S> ForeignFrom<(Secret<T, S>, EncryptDataResponse)> for Encryptable<Secret<T, S>>
where
    T: Clone,
    S: Strategy<T> + Send,
{
    fn foreign_from((masked_data, response): (Secret<T, S>, EncryptDataResponse)) -> Self {
        Self::new(masked_data, response.data.data.peek().clone().into())
    }
}

pub trait DecryptedDataConversion<T: Clone, S: Strategy<T> + Send>: Sized {
    fn convert(
        value: &DecryptedData,
        encryption: Encryption,
    ) -> CustomResult<Self, errors::CryptoError>;
}

impl<S: Strategy<String> + Send> DecryptedDataConversion<String, S>
    for Encryptable<Secret<String, S>>
{
    fn convert(
        value: &DecryptedData,
        encryption: Encryption,
    ) -> CustomResult<Self, errors::CryptoError> {
        let string = String::from_utf8(value.clone().inner().peek().clone()).map_err(|_err| {
            #[cfg(feature = "encryption_service")]
            logger::error!("Decryption error {:?}", _err);
            errors::CryptoError::DecodingFailed
        })?;
        Ok(Self::new(Secret::new(string), encryption.into_inner()))
    }
}

impl<S: Strategy<serde_json::Value> + Send> DecryptedDataConversion<serde_json::Value, S>
    for Encryptable<Secret<serde_json::Value, S>>
{
    fn convert(
        value: &DecryptedData,
        encryption: Encryption,
    ) -> CustomResult<Self, errors::CryptoError> {
        let val = serde_json::from_slice(value.clone().inner().peek()).map_err(|_err| {
            #[cfg(feature = "encryption_service")]
            logger::error!("Decryption error {:?}", _err);
            errors::CryptoError::DecodingFailed
        })?;
        Ok(Self::new(Secret::new(val), encryption.clone().into_inner()))
    }
}

impl<S: Strategy<Vec<u8>> + Send> DecryptedDataConversion<Vec<u8>, S>
    for Encryptable<Secret<Vec<u8>, S>>
{
    fn convert(
        value: &DecryptedData,
        encryption: Encryption,
    ) -> CustomResult<Self, errors::CryptoError> {
        Ok(Self::new(
            Secret::new(value.clone().inner().peek().clone()),
            encryption.clone().into_inner(),
        ))
    }
}

impl<T, S> ForeignTryFrom<(Encryption, DecryptDataResponse)> for Encryptable<Secret<T, S>>
where
    T: Clone,
    S: Strategy<T> + Send,
    Self: DecryptedDataConversion<T, S>,
{
    type Error = error_stack::Report<errors::CryptoError>;
    fn foreign_try_from(
        (encrypted_data, response): (Encryption, DecryptDataResponse),
    ) -> Result<Self, Self::Error> {
        Self::convert(&response.data, encrypted_data)
    }
}

impl<T, S> ForeignTryFrom<(FxHashMap<String, Encryption>, BatchDecryptDataResponse)>
    for FxHashMap<String, Encryptable<Secret<T, S>>>
where
    T: Clone,
    S: Strategy<T> + Send,
    Encryptable<Secret<T, S>>: DecryptedDataConversion<T, S>,
{
    type Error = error_stack::Report<errors::CryptoError>;
    fn foreign_try_from(
        (mut encrypted_data, response): (FxHashMap<String, Encryption>, BatchDecryptDataResponse),
    ) -> Result<Self, Self::Error> {
        response
            .data
            .0
            .into_iter()
            .map(|(k, v)| match encrypted_data.remove(&k) {
                Some(encrypted) => Ok((k.clone(), Encryptable::convert(&v, encrypted.clone())?)),
                None => Err(errors::CryptoError::DecodingFailed)?,
            })
            .collect()
    }
}

impl From<(Encryption, Identifier)> for TransientDecryptDataRequest {
    fn from((encryption, identifier): (Encryption, Identifier)) -> Self {
        Self {
            data: StrongSecret::new(encryption.clone().into_inner().expose()),
            identifier,
        }
    }
}

impl From<(FxHashMap<String, Encryption>, Identifier)> for TransientBatchDecryptDataRequest {
    fn from((map, identifier): (FxHashMap<String, Encryption>, Identifier)) -> Self {
        let data = map
            .into_iter()
            .map(|(k, v)| (k, StrongSecret::new(v.clone().into_inner().expose())))
            .collect();
        Self { data, identifier }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchDecryptDataResponse {
    pub data: DecryptedDataGroup,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptDataResponse {
    pub data: DecryptedData,
}

#[derive(Clone, Debug)]
pub struct DecryptedData(StrongSecret<Vec<u8>>);

impl Serialize for DecryptedData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = BASE64_ENGINE.encode(self.0.peek());
        serializer.serialize_str(&data)
    }
}

impl<'de> Deserialize<'de> for DecryptedData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecryptedDataVisitor;

        impl Visitor<'_> for DecryptedDataVisitor {
            type Value = DecryptedData;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string of the format {version}:{base64_encoded_data}'")
            }

            fn visit_str<E>(self, value: &str) -> Result<DecryptedData, E>
            where
                E: de::Error,
            {
                let dec_data = BASE64_ENGINE.decode(value).map_err(|err| {
                    let err = err.to_string();
                    E::invalid_value(Unexpected::Str(value), &err.as_str())
                })?;

                Ok(DecryptedData(dec_data.into()))
            }
        }

        deserializer.deserialize_str(DecryptedDataVisitor)
    }
}

impl DecryptedData {
    pub fn from_data(data: StrongSecret<Vec<u8>>) -> Self {
        Self(data)
    }

    pub fn inner(self) -> StrongSecret<Vec<u8>> {
        self.0
    }
}

#[derive(Debug)]
pub struct EncryptedData {
    pub data: StrongSecret<Vec<u8>>,
}

impl Serialize for EncryptedData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = String::from_utf8(self.data.peek().clone()).map_err(ser::Error::custom)?;
        serializer.serialize_str(data.as_str())
    }
}

impl<'de> Deserialize<'de> for EncryptedData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EncryptedDataVisitor;

        impl Visitor<'_> for EncryptedDataVisitor {
            type Value = EncryptedData;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("string of the format {version}:{base64_encoded_data}'")
            }

            fn visit_str<E>(self, value: &str) -> Result<EncryptedData, E>
            where
                E: de::Error,
            {
                Ok(EncryptedData {
                    data: StrongSecret::new(value.as_bytes().to_vec()),
                })
            }
        }

        deserializer.deserialize_str(EncryptedDataVisitor)
    }
}

/// A trait which converts the struct to Hashmap required for encryption and back to struct
pub trait ToEncryptable<T, S: Clone, E>: Sized {
    /// Serializes the type to a hashmap
    fn to_encryptable(self) -> FxHashMap<String, E>;
    /// Deserializes the hashmap back to the type
    fn from_encryptable(
        hashmap: FxHashMap<String, Encryptable<S>>,
    ) -> CustomResult<T, errors::ParsingError>;
}
