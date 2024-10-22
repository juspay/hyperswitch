use std::collections::HashSet;

use common_enums::{EntityType, PermissionGroup, Resource, RoleScope};
use common_utils::{errors::CustomResult, id_type};

use super::{permission_groups::PermissionGroupExt, permissions::Permission};
use crate::{core::errors, routes::SessionState};

pub mod predefined_roles;

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct RoleInfo {
    role_id: String,
    role_name: String,
    groups: Vec<PermissionGroup>,
    scope: RoleScope,
    entity_type: EntityType,
    is_invitable: bool,
    is_deletable: bool,
    is_updatable: bool,
    is_internal: bool,
}

impl RoleInfo {
    pub fn get_role_id(&self) -> &str {
        &self.role_id
    }

    pub fn get_role_name(&self) -> &str {
        &self.role_name
    }

    pub fn get_permission_groups(&self) -> Vec<PermissionGroup> {
        self.groups
            .iter()
            .flat_map(|group| group.accessible_groups())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn get_scope(&self) -> RoleScope {
        self.scope
    }

    pub fn get_entity_type(&self) -> EntityType {
        self.entity_type
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

    pub fn get_permissions_set(&self) -> HashSet<Permission> {
        self.get_permission_groups()
            .iter()
            .flat_map(|group| group.permissions_set())
            .collect()
    }

    pub fn get_resources_set(&self) -> HashSet<Resource> {
        self.get_permission_groups()
            .iter()
            .flat_map(|group| group.resources())
            .collect()
    }

    pub fn check_permission_exists(&self, required_permission: &Permission) -> bool {
        required_permission.entity_type() <= self.entity_type
            && self.get_permission_groups().iter().any(|group| {
                required_permission.scope() <= group.scope()
                    && group.resources().contains(&required_permission.resource())
            })
    }

    pub async fn from_role_id_in_merchant_scope(
        state: &SessionState,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<Self, errors::StorageError> {
        if let Some(role) = predefined_roles::PREDEFINED_ROLES.get(role_id) {
            Ok(role.clone())
        } else {
            state
                .store
                .find_role_by_role_id_in_merchant_scope(role_id, merchant_id, org_id)
                .await
                .map(Self::from)
        }
    }

    pub async fn from_role_id_in_org_scope(
        state: &SessionState,
        role_id: &str,
        org_id: &id_type::OrganizationId,
    ) -> CustomResult<Self, errors::StorageError> {
        if let Some(role) = predefined_roles::PREDEFINED_ROLES.get(role_id) {
            Ok(role.clone())
        } else {
            state
                .store
                .find_role_by_role_id_in_org_scope(role_id, org_id)
                .await
                .map(Self::from)
        }
    }
}

impl From<diesel_models::role::Role> for RoleInfo {
    fn from(role: diesel_models::role::Role) -> Self {
        Self {
            role_id: role.role_id,
            role_name: role.role_name,
            groups: role.groups.into_iter().map(Into::into).collect(),
            scope: role.scope,
            entity_type: role.entity_type.unwrap_or(EntityType::Merchant),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        }
    }
}
