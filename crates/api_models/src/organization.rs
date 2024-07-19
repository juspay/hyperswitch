use common_utils::id_type;
use utoipa::ToSchema;
pub struct OrganizationNew {
    pub org_id: id_type::OrganizationId,
    pub org_name: Option<String>,
}

impl OrganizationNew {
    pub fn new(org_name: Option<String>) -> Self {
        Self {
            org_id: id_type::OrganizationId::default(),
            org_name,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct OrganizationId {
    pub organization_id: id_type::OrganizationId,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct OrganizationRequest {
    pub organization_name: Option<String>,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct OrganizationResponse {
    pub organization_id: id_type::OrganizationId,
    pub organization_name: Option<String>,
}
