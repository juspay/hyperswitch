use base64::Engine;
use bytes::Bytes;
use common_utils::ext_traits::BytesExt;
use diesel_models::encryption::Encryption;
use masking::{ExposeInterface, PeekInterface, Secret, Strategy, StrongSecret};
use rdkafka::message::ToBytes;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

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
            String::from_utf8_lossy(secret.peek()).to_string(),
            DecryptedData(StrongSecret::new(secret.expose())),
        );
        Self {
            identifier,
            data: DecryptedDataGroup(group),
        }
    }
}

impl<S> From<(Vec<Secret<Vec<u8>, S>>, Identifier)> for EncryptDataRequest
where
    S: Strategy<Vec<u8>>,
{
    fn from((vec, identifier): (Vec<Secret<Vec<u8>, S>>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        for item in vec.iter() {
            group.insert(
                String::from_utf8_lossy(item.clone().peek()).to_string(),
                DecryptedData(StrongSecret::new(item.clone().expose())),
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
        let exposed = secret.clone().expose();
        group.insert(
            exposed.clone(),
            DecryptedData(StrongSecret::new(exposed.as_bytes().to_vec())),
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
        let exposed = secret.clone().expose();
        group.insert(
            exposed.clone().to_string(),
            DecryptedData(StrongSecret::new(exposed.to_string().as_bytes().to_vec())),
        );
        Self {
            data: DecryptedDataGroup(group),
            identifier: identifier.clone(),
        }
    }
}

impl<S> From<(Vec<Secret<serde_json::Value, S>>, Identifier)> for EncryptDataRequest
where
    S: Strategy<serde_json::Value>,
{
    fn from((vec, identifier): (Vec<Secret<serde_json::Value, S>>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        for item in vec.into_iter() {
            let exposed = item.clone().expose();
            group.insert(
                exposed.clone().to_string(),
                DecryptedData(StrongSecret::new(exposed.to_string().as_bytes().to_vec())),
            );
        }
        Self {
            data: DecryptedDataGroup(group),
            identifier,
        }
    }
}

impl<S> From<(Vec<Secret<String, S>>, Identifier)> for EncryptDataRequest
where
    S: Strategy<String>,
{
    fn from((vec, identifier): (Vec<Secret<String, S>>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        for item in vec.into_iter() {
            let exposed = item.clone().expose();
            group.insert(
                exposed.clone(),
                DecryptedData(StrongSecret::new(exposed.as_bytes().to_vec())),
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
    type Error = error_stack::Report<common_utils::errors::ParsingError>;
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
        let exposed = encryption.clone().into_inner().expose();
        group.insert(
            String::from_utf8_lossy(exposed.to_bytes()).to_string(),
            EncryptedData {
                data: StrongSecret::new(exposed),
            },
        );
        Self {
            data: EncryptedDataGroup(group),
            identifier,
        }
    }
}

impl From<(Vec<Encryption>, Identifier)> for DecryptDataRequest {
    fn from((vec, identifier): (Vec<Encryption>, Identifier)) -> Self {
        let mut group = FxHashMap::default();
        for item in vec.into_iter() {
            let exposed = item.clone().into_inner().expose();
            group.insert(
                String::from_utf8_lossy(exposed.to_bytes()).to_string(),
                EncryptedData {
                    data: StrongSecret::new(exposed),
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
    type Error = error_stack::Report<common_utils::errors::ParsingError>;
    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        value.parse_struct::<Self>("DecryptDataResponse")
    }
}

#[derive(Clone, Debug, Deserialize)]
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

impl DecryptedData {
    pub fn from_data(data: StrongSecret<Vec<u8>>) -> Self {
        Self(data)
    }

    pub fn inner(self) -> StrongSecret<Vec<u8>> {
        self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedData {
    pub data: StrongSecret<Vec<u8>>,
}
