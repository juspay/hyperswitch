#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateCardResponse {
    pub status_message: String,
    pub status_code: String,
    pub customers_moved: usize,
    pub cards_moved: usize,
}
