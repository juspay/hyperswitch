use common_enums::{PermissionGroup, RoleScope};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CreateRoleRequest {
    pub role_name: String,
    pub groups: Vec<PermissionGroup>,
    pub scope: RoleScope,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateRoleRequest {
    pub groups: Option<Vec<PermissionGroup>>,
    pub role_name: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ListRolesResponse(pub Vec<RoleInfoResponse>);

#[derive(Debug, serde::Serialize)]
pub struct RoleInfoResponse {
    pub role_id: String,
    pub permissions: Vec<super::Permission>,
    pub role_name: String,
    pub role_scope: RoleScope,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetRoleRequest {
    pub role_id: String,
}
