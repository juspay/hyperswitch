use common_utils::id_type;
use time::PrimitiveDateTime;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct GenerateResourceRequest {
    #[serde(rename = "type")]
    pub resource_type: common_enums::ResourceType,
    pub apple_merchant_identifier: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct GenerateResourceResponse {
    pub id: id_type::ResourceId,
    #[schema(value_type = Object)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
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
pub struct ResourceSummary {
    pub id: id_type::ResourceId,
    #[schema(value_type = Object)]
    pub display_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub display_data: serde_json::Value,
    #[schema(value_type = String)]
    pub created_at: PrimitiveDateTime,
    pub is_linked: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct ListResourcesRequest {
    #[serde(rename = "type")]
    pub resource_type: common_enums::ResourceType,
    pub scope_id: Option<String>,
    pub scope_type: Option<common_enums::ResourceRequestorType>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ListResourcesResponse {
    pub resources: Vec<ResourceSummary>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct LinkResourceRequest {
    pub requestor_type: common_enums::ResourceRequestorType,
    pub requestor_id: String,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LinkResourceResponse {
    pub id: id_type::ResourceId,
}
