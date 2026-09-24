use common_utils::id_type;
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateHierarchicalResourceRequest {
    #[serde(rename = "type")]
    pub resource_type: common_enums::ResourceType,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct GenerateHierarchicalResourceResponse {
    pub id: id_type::ResourceId,
    #[schema(value_type = Object)]
    pub data: Secret<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadCertificateRequest {
    pub certificate: String,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct UploadCertificateResponse {
    pub id: id_type::ResourceId,
    #[schema(value_type = String)]
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct HierarchicalResourceSummary {
    pub id: id_type::ResourceId,
    #[schema(value_type = Object)]
    pub display_schema: Secret<serde_json::Value>,
    #[schema(value_type = Object)]
    pub display_data: Secret<serde_json::Value>,
    #[schema(value_type = String)]
    pub created_at: PrimitiveDateTime,
    pub is_linked: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListHierarchicalResourcesRequest {
    #[serde(rename = "type")]
    pub resource_type: common_enums::ResourceType,
    pub scope_id: Option<String>,
    pub scope_type: Option<common_enums::ResourceRequestorType>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ListHierarchicalResourcesResponse {
    pub resources: Vec<HierarchicalResourceSummary>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkHierarchicalResourceRequest {
    pub requestor_type: common_enums::ResourceRequestorType,
    pub requestor_id: String,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LinkHierarchicalResourceResponse {
    pub id: id_type::ResourceId,
}
