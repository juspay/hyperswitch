use super::Permission;
use common_enums::{PermissionGroup, RoleScope};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CreateRoleRequest {
    pub role_name: String,
    pub groups: Vec<PermissionGroup>,
    pub role_scope: RoleScope,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "response_type", rename_all = "snake_case")]
pub enum ListRolesResponse {
    Permissions(Vec<RoleInfoWithPermissionsResponse>),
    Groups(Vec<RoleInfoWithGroupsResponse>),
}

#[derive(Debug, serde::Deserialize)]
pub struct GetGroupsQueryParam {
    pub groups: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "response_type", rename_all = "snake_case")]
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
pub struct UpdateRoleRequest {
    pub groups: Option<Vec<PermissionGroup>>,
    pub role_name: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetRoleRequest {
    pub role_id: String,
}
