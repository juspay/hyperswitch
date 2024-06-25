use core::fmt;

use base64::Engine;
use bytes::Bytes;
use common_utils::{errors, ext_traits::BytesExt};
use diesel_models::encryption::Encryption;
use masking::{ExposeInterface, PeekInterface, Secret, Strategy, StrongSecret};
use rustc_hash::FxHashMap;
use serde::{
    de::{self, Unexpected, Visitor},
    ser, Deserialize, Deserializer, Serialize,
};

use crate::consts::BASE64_ENGINE;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(tag = "data_identifier", content = "key_identifier")]
pub enum Identifier {
    User(String),
    Merchant(String),
}

impl Identifier {
    pub fn inner(&self) -> &String {
        match self {
            Self::User(id) => id,
            Self::Merchant(id) => id,
        }
    }
}

pub const DEFAULT_KEY: &str = "DEFAULT";

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct EncryptionCreateRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptedDataGroup,
}

impl<S> From<(Secret<Vec<u8>, S>, Identifier)> for EncryptDataRequest
where
    S: Strategy<Vec<u8>>,
{
    fn from((secret, identifier): (Secret<Vec<u8>, S>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        group.insert(
            DEFAULT_KEY.to_string(),
            DecryptedData(StrongSecret::new(secret.expose())),
        );
        Self {
            identifier,
            data: DecryptedDataGroup(group),
        }
    }
}

impl<S> From<(FxHashMap<String, Secret<Vec<u8>, S>>, Identifier)> for EncryptDataRequest
where
    S: Strategy<Vec<u8>>,
{
    fn from((map, identifier): (FxHashMap<String, Secret<Vec<u8>, S>>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        for (key, value) in map.iter() {
            group.insert(
                key.clone(),
                DecryptedData(StrongSecret::new(value.clone().expose())),
            );
        }
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
        let mut group = FxHashMap::default();
        group.insert(
            DEFAULT_KEY.to_string(),
            DecryptedData(StrongSecret::new(
                secret.clone().expose().as_bytes().to_vec(),
            )),
        );
        Self {
            data: DecryptedDataGroup(group),
            identifier: identifier.clone(),
        }
    }
}

impl<S> From<(Secret<serde_json::Value, S>, Identifier)> for EncryptDataRequest
where
    S: Strategy<serde_json::Value>,
{
    fn from((secret, identifier): (Secret<serde_json::Value, S>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        group.insert(
            DEFAULT_KEY.to_string(),
            DecryptedData(StrongSecret::new(
                secret.clone().expose().to_string().as_bytes().to_vec(),
            )),
        );
        Self {
            data: DecryptedDataGroup(group),
            identifier: identifier.clone(),
        }
    }
}

impl<S> From<(FxHashMap<String, Secret<serde_json::Value, S>>, Identifier)> for EncryptDataRequest
where
    S: Strategy<serde_json::Value>,
{
    fn from(
        (map, identifier): (FxHashMap<String, Secret<serde_json::Value, S>>, Identifier),
    ) -> Self {
        let mut group = FxHashMap::default();
        for (key, value) in map.into_iter() {
            group.insert(
                key.clone(),
                DecryptedData(StrongSecret::new(
                    value.clone().expose().to_string().as_bytes().to_vec(),
                )),
            );
        }
        Self {
            data: DecryptedDataGroup(group),
            identifier,
        }
    }
}

impl<S> From<(FxHashMap<String, Secret<String, S>>, Identifier)> for EncryptDataRequest
where
    S: Strategy<String>,
{
    fn from((map, identifier): (FxHashMap<String, Secret<String, S>>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        for (key, value) in map.into_iter() {
            group.insert(
                key.clone(),
                DecryptedData(StrongSecret::new(
                    value.clone().expose().as_bytes().to_vec(),
                )),
            );
        }
        Self {
            data: DecryptedDataGroup(group),
            identifier,
        }
    }
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct DecryptedDataGroup(pub FxHashMap<String, DecryptedData>);

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptDataResponse {
    pub data: EncryptedDataGroup,
}

impl TryFrom<Bytes> for EncryptDataResponse {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        value.parse_struct::<Self>("EncryptDataResponse")
    }
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct EncryptedDataGroup(pub FxHashMap<String, EncryptedData>);
#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: EncryptedDataGroup,
}

impl From<(Encryption, Identifier)> for DecryptDataRequest {
    fn from((encryption, identifier): (Encryption, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        group.insert(
            DEFAULT_KEY.to_string(),
            EncryptedData {
                data: StrongSecret::new(encryption.clone().into_inner().expose()),
            },
        );
        Self {
            data: EncryptedDataGroup(group),
            identifier,
        }
    }
}

impl From<(FxHashMap<String, Encryption>, Identifier)> for DecryptDataRequest {
    fn from((map, identifier): (FxHashMap<String, Encryption>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        for (key, value) in map.into_iter() {
            group.insert(
                key,
                EncryptedData {
                    data: StrongSecret::new(value.clone().into_inner().expose()),
                },
            );
        }
        Self {
            data: EncryptedDataGroup(group),
            identifier,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecryptDataResponse {
    pub data: DecryptedDataGroup,
}

impl TryFrom<Bytes> for DecryptDataResponse {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        value.parse_struct::<Self>("DecryptDataResponse")
    }
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

        impl<'de> Visitor<'de> for DecryptedDataVisitor {
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

        impl<'de> Visitor<'de> for EncryptedDataVisitor {
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
