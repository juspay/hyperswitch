#[derive(Debug, Clone, serde::Serialize)]
pub struct ApplePayCertificatesMigrationResponse {
    pub migration_successful: Vec<String>,
    pub migration_failed: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ApplePayCertificatesMigrationRequest {
    pub merchant_ids: Vec<common_utils::id_type::MerchantId>,
}

impl common_utils::events::ApiEventMetric for ApplePayCertificatesMigrationRequest {}
