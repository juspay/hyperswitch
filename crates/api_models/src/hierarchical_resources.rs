use common_enums::{ResourceRequestorType, ResourceType};
use common_utils::id_type;
use hyperswitch_masking::Secret;
use time::PrimitiveDateTime;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateHierarchicalResourceRequest {
    #[serde(rename = "type")]
    pub resource_type: ResourceType,
    pub google_pay: Option<GooglePayResourceRequest>,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GooglePayResourceMode {
    Direct,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize, utoipa::ToSchema,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GooglePayResourceEnvironment {
    Test,
    Production,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GooglePayResourceRequest {
    pub mode: GooglePayResourceMode,
    pub environment: GooglePayResourceEnvironment,
    pub google_merchant_id: String,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct GenerateHierarchicalResourceResponse {
    #[schema(value_type = String)]
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
    #[schema(value_type = String)]
    pub id: id_type::ResourceId,
    #[schema(value_type = String)]
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct HierarchicalResourceSummary {
    #[schema(value_type = String)]
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
    pub resource_type: ResourceType,
    pub scope_id: Option<String>,
    pub scope_type: Option<ResourceRequestorType>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ListHierarchicalResourcesResponse {
    pub resources: Vec<HierarchicalResourceSummary>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkHierarchicalResourceRequest {
    pub requestor_type: ResourceRequestorType,
    pub requestor_id: String,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LinkHierarchicalResourceResponse {
    #[schema(value_type = String)]
    pub id: id_type::ResourceId,
}
