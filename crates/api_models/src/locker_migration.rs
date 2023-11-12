#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateCardResponse {
    pub status_message: String,
    pub status_code: String,
}
