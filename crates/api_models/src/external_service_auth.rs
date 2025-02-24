use common_utils::{id_type, pii};
use masking::Secret;

#[derive(Debug, serde::Serialize)]
pub struct ExternalTokenResponse {
    pub token: Secret<String>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExternalVerifyTokenRequest {
    pub token: Secret<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExternalSignoutTokenRequest {
    pub token: Secret<String>,
}

#[derive(serde::Serialize, Debug)]
#[serde(untagged)]
pub enum ExternalVerifyTokenResponse {
    Hypersense {
        user_id: String,
        merchant_id: id_type::MerchantId,
        name: Secret<String>,
        email: pii::Email,
    },
}

impl ExternalVerifyTokenResponse {
    pub fn get_user_id(&self) -> &str {
        match self {
            Self::Hypersense { user_id, .. } => user_id,
        }
    }
}
