#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

#[cfg(feature = "keymanager_mtls")]
use masking::Secret;

#[derive(Debug)]
pub struct KeyManagerState {
    pub url: String,
    pub client_idle_timeout: Option<u64>,
    #[cfg(feature = "keymanager_mtls")]
    pub ca: Secret<String>,
    #[cfg(feature = "keymanager_mtls")]
    pub cert: Secret<String>,
}
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

#[derive(Debug, Deserialize, Serialize)]
pub struct DataKeyCreateResponse {
    #[serde(flatten)]
    pub identifier: Identifier,
    pub key_version: String,
}
