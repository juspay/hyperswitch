use common_utils::{id_type, pii};
use hyperswitch_masking::Secret;

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

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ValidatingService {
    OfferEngine,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ValidateTokenRequest {
    pub token: Secret<String>,
    pub service: ValidatingService,
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
    OfferEngine {
        merchant_id: String,
        context: String,
        token: Secret<String>,
        permissions: Vec<String>,
    },
}

impl ExternalVerifyTokenResponse {
    pub fn get_user_id(&self) -> Option<&str> {
        match self {
            Self::Hypersense { user_id, .. } => Some(user_id),
            Self::OfferEngine { .. } => None,
        }
    }
}
