use common_utils::id_type;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AutomationAiGetDataRequest {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub org_id: id_type::OrganizationId,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct GetDataMessage {
    pub message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct EmbeddedAiGetDataRequest {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub org_id: id_type::OrganizationId,
    pub query: GetDataMessage,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct EmbeddedAiDataResponse {
    pub response: serde_json::Value,
    pub merchant_id: id_type::MerchantId,
    pub status: String,
    pub query_executed: Option<String>,
    pub row_count: Option<i32>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Output {
    pub summary: String,
    pub markdown: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AutomationAiDataResponse {
    pub output: Output,
}
