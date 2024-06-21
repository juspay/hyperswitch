use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
#[serde(tag = "data_identifier", content = "key_identifier")]
pub enum Identifier {
    User(String),
    Merchant(String),
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
