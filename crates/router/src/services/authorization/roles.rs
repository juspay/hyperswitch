#[cfg(feature = "recon")]
use std::collections::HashMap;
use std::collections::HashSet;

#[cfg(feature = "recon")]
use api_models::enums::ReconPermissionScope;
use common_enums::{EntityType, PermissionGroup, Resource, RoleScope};
use common_utils::{errors::CustomResult, id_type};

#[cfg(feature = "recon")]
use super::permission_groups::{RECON_OPS, RECON_REPORTS};
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

    pub fn get_resources_set(&self) -> HashSet<Resource> {
        self.get_permission_groups()
            .iter()
            .flat_map(|group| group.resources())
            .collect()
    }

    pub fn check_permission_exists(&self, required_permission: Permission) -> bool {
        required_permission.entity_type() <= self.entity_type
            && self.get_permission_groups().iter().any(|group| {
                required_permission.scope() <= group.scope()
                    && group.resources().contains(&required_permission.resource())
            })
    }

    #[cfg(feature = "recon")]
    pub fn get_recon_acl(&self) -> HashMap<Resource, ReconPermissionScope> {
        let mut acl: HashMap<Resource, ReconPermissionScope> = HashMap::new();
        let mut recon_resources = RECON_OPS.to_vec();
        recon_resources.extend(RECON_REPORTS);
        let recon_internal_resources = [Resource::ReconToken];
        self.get_permission_groups()
            .iter()
            .for_each(|permission_group| {
                permission_group.resources().iter().for_each(|resource| {
                    if recon_resources.contains(resource)
                        && !recon_internal_resources.contains(resource)
                    {
                        let scope = match resource {
                            Resource::ReconAndSettlementAnalytics => ReconPermissionScope::Read,
                            _ => ReconPermissionScope::from(permission_group.scope()),
                        };
                        acl.entry(*resource)
                            .and_modify(|curr_scope| {
                                *curr_scope = if (*curr_scope) < scope {
                                    scope
                                } else {
                                    *curr_scope
                                }
                            })
                            .or_insert(scope);
                    }
                })
            });
        acl
    }

    pub async fn from_role_id_in_lineage(
        state: &SessionState,
        role_id: &str,
        merchant_id: &id_type::MerchantId,
        org_id: &id_type::OrganizationId,
        profile_id: &id_type::ProfileId,
        tenant_id: &id_type::TenantId,
    ) -> CustomResult<Self, errors::StorageError> {
        if let Some(role) = predefined_roles::PREDEFINED_ROLES.get(role_id) {
            Ok(role.clone())
        } else {
            state
                .global_store
                .find_role_by_role_id_in_lineage(
                    role_id,
                    merchant_id,
                    org_id,
                    profile_id,
                    tenant_id,
                )
                .await
                .map(Self::from)
        }
    }

    // TODO: To evaluate whether we can omit org_id and tenant_id for this function
    pub async fn from_role_id_org_id_tenant_id(
        state: &SessionState,
        role_id: &str,
        org_id: &id_type::OrganizationId,
        tenant_id: &id_type::TenantId,
    ) -> CustomResult<Self, errors::StorageError> {
        if let Some(role) = predefined_roles::PREDEFINED_ROLES.get(role_id) {
            Ok(role.clone())
        } else {
            state
                .global_store
                .find_by_role_id_org_id_tenant_id(role_id, org_id, tenant_id)
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
            groups: role.groups,
            scope: role.scope,
            entity_type: role.entity_type,
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
            is_internal: false,
        }
    }
}
