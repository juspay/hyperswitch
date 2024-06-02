#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayCertificatesMigrationResponse {
    pub migration_sucessful: Vec<String>,
    pub migraiton_failed: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ApplePayCertificatesMigrationRequest {
    pub merchant_ids: Vec<String>,
}

impl common_utils::events::ApiEventMetric for ApplePayCertificatesMigrationRequest {}
