use common_utils::id_type;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AutomationAiDataRequest {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub org_id: id_type::OrganizationId,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct GetDataMessage {
    pub message: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct EmbeddedAiDataRequest {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub org_id: id_type::OrganizationId,
    pub query: GetDataMessage,
}
