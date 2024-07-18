use utoipa::ToSchema;
pub struct OrganizationNew {
    pub org_id: String,
    pub org_name: Option<String>,
}

impl OrganizationNew {
    pub fn new(org_name: Option<String>) -> Self {
        Self {
            org_id: common_utils::generate_id_with_default_len("org"),
            org_name,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct OrganizationId {
    pub organization_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
pub struct OrganizationRequest {
    pub organization_name: Option<String>,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct OrganizationResponse {
    pub organization_id: String,
    pub organization_name: Option<String>,
}
