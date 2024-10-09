pub use common_enums::PermissionGroup;
use common_enums::{EntityType, RoleScope};

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
pub struct RoleInfoWithGroupsResponse {
    pub role_id: String,
    pub groups: Vec<PermissionGroup>,
    pub role_name: String,
    pub role_scope: RoleScope,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ListRolesRequest {
    pub entity_type: Option<EntityType>,
}

#[derive(Debug, serde::Serialize)]
pub struct RoleInfoResponseNew {
    pub role_id: String,
    pub role_name: String,
    pub entity_type: EntityType,
    pub groups: Vec<PermissionGroup>,
    pub scope: RoleScope,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetRoleRequest {
    pub role_id: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ListRolesAtEntityLevelRequest {
    pub entity_type: EntityType,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum RoleCheckType {
    Invite,
    Update,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct MinimalRoleInfo {
    pub role_id: String,
    pub role_name: String,
}
