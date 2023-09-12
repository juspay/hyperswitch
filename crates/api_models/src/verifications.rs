/// The request body for verification of merchant (everything except domain_names are prefilled)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayMerchantVerificationConfigs {
    pub domain_names: Vec<String>,
    pub encrypt_to: String,
    pub partner_internal_merchant_identifier: String,
    pub partner_merchant_name: String,
}

/// The derivation point for domain names from request body
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayMerchantVerificationRequest {
    pub domain_names: Vec<String>,
    pub business_profile_id: String,
}

/// Response to be sent for the verify/applepay api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplepayMerchantResponse {
    pub status_message: String,
    pub status_code: String,
}
