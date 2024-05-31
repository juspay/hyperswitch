use masking::Secret;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleRequest {
    ApiKey {
        // This is hardcoded (supposed to be tenant id)
        tag: String,
        api_key: Secret<String>,
        identifiers: ApiKeyIdentifier,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiKeyIdentifier {
    ApiKey { merchant_id: String, key_id: String },
    PublishableKey { merchant_id: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RuleResponse {
    Added {
        tag: String,
        #[serde(flatten)]
        identifiers: ApiKeyIdentifier,
    },
}
