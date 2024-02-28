use common_enums::{PermissionGroup, RoleScope};

use super::Permission;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CreateRoleRequest {
    pub role_name: String,
    pub groups: Vec<PermissionGroup>,
    pub role_scope: RoleScope,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateRoleRequest {
    pub groups: Option<Vec<PermissionGroup>>,
    pub role_name: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ListRolesResponse(pub Vec<RoleInfoResponse>);

#[derive(Debug, serde::Deserialize)]
pub struct GetGroupsQueryParam {
    pub groups: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum GetRoleFromTokenResponse {
    Permissions(Vec<Permission>),
    Groups(Vec<PermissionGroup>),
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum RoleInfoResponse {
    Permissions(RoleInfoWithPermissionsResponse),
    Groups(RoleInfoWithGroupsResponse),
}

#[derive(Debug, serde::Serialize)]
pub struct RoleInfoWithPermissionsResponse {
    pub role_id: String,
    pub permissions: Vec<Permission>,
    pub role_name: String,
    pub role_scope: RoleScope,
}

#[derive(Debug, serde::Serialize)]
pub struct RoleInfoWithGroupsResponse {
    pub role_id: String,
    pub groups: Vec<PermissionGroup>,
    pub role_name: String,
    pub role_scope: RoleScope,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetRoleRequest {
    pub role_id: String,
}
