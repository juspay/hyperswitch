use common_utils::id_type;
use masking::Secret;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatRequest {
    pub message: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatResponse {
    pub response: Secret<serde_json::Value>,
    pub merchant_id: id_type::MerchantId,
    pub status: String,
    #[serde(skip_serializing)]
    pub query_executed: Option<Secret<String>>,
    #[serde(skip_serializing)]
    pub row_count: Option<i32>,
}
