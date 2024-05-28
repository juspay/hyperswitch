#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayCertificatesMigrationResponse {
    pub status_message: String,
    pub status_code: String,
}
