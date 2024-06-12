use core::fmt;
use std::fmt::Display;

use base64::Engine;
use masking::PeekInterface;
use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize,
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

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct EncryptionCreateRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptedData,
}

#[derive(Serialize, Deserialize)]
pub struct EncryptDataResponse {
    pub data: EncryptedData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: EncryptedData,
}

#[derive(Serialize, Deserialize)]
pub struct DecryptDataResponse {
    pub data: DecryptedData,
}

#[derive(Debug)]
pub struct DecryptedData(masking::StrongSecret<Vec<u8>>);

impl DecryptedData {
    pub fn from_data(data: masking::StrongSecret<Vec<u8>>) -> Self {
        Self(data)
    }

    pub fn inner(self) -> masking::StrongSecret<Vec<u8>> {
        self.0
    }
}

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

#[derive(Debug)]
pub struct EncryptedData {
    pub version: Version,
    pub data: masking::StrongSecret<Vec<u8>>,
}

impl EncryptedData {
    pub fn inner(self) -> masking::StrongSecret<Vec<u8>> {
        self.data
    }
}

impl Serialize for EncryptedData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data = BASE64_ENGINE.encode(self.data.peek());
        let encoded = format!("{}:{}", &self.version, data);
        serializer.serialize_str(&encoded)
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
                let (version, data) = value.split_once(':').ok_or_else(|| {
                    E::invalid_value(
                        Unexpected::Str(value),
                        &"String should of the format {version}:{base64_encoded_data}",
                    )
                })?;

                let dec_data = BASE64_ENGINE.decode(data).map_err(|err| {
                    let err = err.to_string();
                    E::invalid_value(Unexpected::Str(data), &err.as_str())
                })?;

                Ok(EncryptedData {
                    version: Version::from(version.to_string()),
                    data: masking::StrongSecret::new(dec_data),
                })
            }
        }

        deserializer.deserialize_str(EncryptedDataVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Version(String);

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self("v1".to_string())
    }
}

impl From<String> for Version {
    fn from(v: String) -> Self {
        Self(v)
    }
}
