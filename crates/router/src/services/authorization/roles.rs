use super::{permission_groups::PermissionGroup, permissions::Permission};
use crate::{core::errors, routes::AppState};
use common_utils::errors::CustomResult;

pub mod predefined_roles;

#[allow(dead_code)]
#[derive(Clone)]
pub struct RoleInfo {
    role_id: String,
    role_name: String,
    groups: Vec<PermissionGroup>,
    is_invitable: bool,
    is_deletable: bool,
    is_updatable: bool,
    is_internal: bool,
}

impl RoleInfo {
    pub fn get_role_id(&self) -> &str {
        &self.role_id
    }

    pub fn get_name(&self) -> &str {
        &self.role_name
    }

    pub fn get_permission_groups(&self) -> &Vec<PermissionGroup> {
        &self.groups
    }

    pub fn is_invitable(&self) -> bool {
        self.is_invitable
    }

    pub fn is_deletable(&self) -> bool {
        self.is_deletable
    }

    pub fn is_internal(&self) -> bool {
        self.is_internal
    }

    pub fn is_updatable(&self) -> bool {
        self.is_updatable
    }

    pub fn get_permissions(&self) -> Vec<Permission> {
        self.groups
            .iter()
            .flat_map(|group| group.get_permissions_vec().iter().copied())
            .collect()
    }

    pub fn check_permission_exists(&self, required_permission: &Permission) -> bool {
        self.groups
            .iter()
            .any(|module| module.get_permissions_vec().contains(required_permission))
    }
}

pub async fn get_role_info_from_role_id(
    state: &AppState,
    role_id: &str,
) -> CustomResult<RoleInfo, errors::StorageError> {
    if let Some(role) = predefined_roles::PREDEFINED_ROLES.get(role_id) {
        Ok(role.clone())
    } else {
        let role = state.store.find_role_by_role_id(role_id).await?;
        Ok(role.into())
    }
}

impl From<diesel_models::role::Role> for RoleInfo {
    fn from(role: diesel_models::role::Role) -> Self {
        RoleInfo {
            groups: role.groups.into_iter().map(Into::into).collect(),
            role_name: role.role_name,
            role_id: role.role_id,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        }
    }
}
