use common_enums::{ParentGroup, PermissionGroup};
use common_utils::pii;
use masking::Secret;

pub mod role;

#[derive(Debug, serde::Serialize)]
pub struct AuthorizationInfoResponse(pub Vec<AuthorizationInfo>);

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum AuthorizationInfo {
    Group(GroupInfo),
    GroupWithTag(ParentInfo),
}

// TODO: To be deprecated
#[derive(Debug, serde::Serialize)]
pub struct GroupInfo {
    pub group: PermissionGroup,
    pub description: &'static str,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct ParentInfo {
    pub name: ParentGroup,
    pub description: &'static str,
    pub groups: Vec<PermissionGroup>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateUserRoleRequest {
    pub email: pii::Email,
    pub role_id: String,
}

#[derive(Debug, serde::Serialize)]
pub enum UserStatus {
    Active,
    InvitationSent,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DeleteUserRoleRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Serialize)]
pub struct ListUsersInEntityResponse {
    pub email: pii::Email,
    pub roles: Vec<role::MinimalRoleInfo>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ListInvitationForUserResponse {
    pub entity_id: String,
    pub entity_type: common_enums::EntityType,
    pub entity_name: Option<Secret<String>>,
    pub role_id: String,
}

pub type AcceptInvitationsV2Request = Vec<Entity>;
pub type AcceptInvitationsPreAuthRequest = Vec<Entity>;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub entity_id: String,
    pub entity_type: common_enums::EntityType,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ListUsersInEntityRequest {
    pub entity_type: Option<common_enums::EntityType>,
}
