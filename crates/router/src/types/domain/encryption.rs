use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
pub struct EncryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: DecryptedData,
}

#[derive(Serialize, Deserialize)]
pub struct EncryptDataResponse {
    pub data: EncryptedData,
}

#[derive(Serialize, Deserialize)]
pub struct DecryptDataRequest {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub data: EncryptedData,
}

#[derive(Serialize, Deserialize)]
pub struct DecryptDataResponse {
    pub data: DecryptedData,
}

#[derive(Deserialize, Serialize)]
pub struct DecryptedData(masking::StrongSecret<Vec<u8>>);

impl DecryptedData {
    pub fn from_data(data: masking::StrongSecret<Vec<u8>>) -> Self {
        Self(data)
    }

    pub fn inner(self) -> masking::StrongSecret<Vec<u8>> {
        self.0
    }
}

#[derive(Deserialize, Serialize)]
pub struct EncryptedData {
    pub version: Version,
    pub data: masking::StrongSecret<Vec<u8>>,
}

impl EncryptedData {
    pub fn inner(self) -> masking::StrongSecret<Vec<u8>> {
        self.data
    }
}

#[derive(Serialize, Deserialize)]
pub struct Version(String);

impl From<String> for Version {
    fn from(v: String) -> Self {
        Self(v)
    }
}
