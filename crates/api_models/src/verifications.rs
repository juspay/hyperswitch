#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayMerchantVerificationConfigs {
    pub domain_names: Vec<String>,
    pub encrypt_to: String,
    pub partner_internal_merchant_identifier: String,
    pub partner_merchant_name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayMerchantVerificationRequest {
    pub domain_names: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayMerchantResponse {
    pub status_message: String,
    pub status_code: String,
}
