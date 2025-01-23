use common_utils::{id_type, pii};
use masking::Secret;

#[derive(Debug, serde::Serialize)]
pub struct HypersenseTokenResponse {
    pub token: Secret<String>,
}
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HypersenseVerifyTokenRequest {
    pub token: Secret<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HypersenseLogoutTokenRequest {
    pub token: Secret<String>,
}

#[derive(serde::Serialize, Debug)]
pub struct HypersenseVerifyTokenResponse {
    pub user_id: String,
    pub merchant_id: id_type::MerchantId,
    pub name: Secret<String>,
    pub email: pii::Email,
}
