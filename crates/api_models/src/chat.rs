use common_utils::id_type;
use masking::Secret;
use time::PrimitiveDateTime;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatRequest {
    pub message: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ChatResponse {
    pub response: Secret<serde_json::Value>,
    pub merchant_id: id_type::MerchantId,
    pub status: String,
    #[serde(skip_serializing)]
    pub query_executed: Option<Secret<String>>,
    #[serde(skip_serializing)]
    pub row_count: Option<i32>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatListRequest {
    pub merchant_id: Option<id_type::MerchantId>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatConversation {
    pub id: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub merchant_id: Option<String>,
    pub profile_id: Option<String>,
    pub org_id: Option<String>,
    pub role_id: Option<String>,
    pub user_query: Secret<String>,
    pub response: Secret<serde_json::Value>,
    pub database_query: Option<String>,
    pub interaction_status: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ChatListResponse {
    pub conversations: Vec<ChatConversation>,
}
