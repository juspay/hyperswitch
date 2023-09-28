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
#[serde(rename_all = "snake_case")]
pub struct ApplepayMerchantVerificationRequest {
    pub domain_names: Vec<String>,
    pub merchant_connector_account_id: String,
}

/// Response to be sent for the verify/applepay api
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApplepayMerchantResponse {
    pub status_message: String,
}

/// QueryParams to be send by the merchant for fetching the verified domains
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApplepayGetVerifiedDomainsParam {
    pub merchant_id: String,
    pub merchant_connector_account_id: String,
}
/// Response to be sent for derivation of the already verified domains
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApplepayVerifiedDomainsResponse {
    pub verified_domains: Vec<String>,
}
