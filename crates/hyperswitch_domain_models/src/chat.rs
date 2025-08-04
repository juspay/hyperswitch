use common_utils::id_type;
use masking::Secret;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct GetDataMessage {
    pub message: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct HyperswitchAiDataRequest {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
    pub org_id: id_type::OrganizationId,
    pub query: GetDataMessage,
    pub entity_type: common_enums::EntityType,
}
