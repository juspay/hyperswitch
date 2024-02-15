use std::collections::HashMap;

#[cfg(feature = "olap")]
use error_stack::ResultExt;
use once_cell::sync::Lazy;

use super::{permission_groups::PermissionGroup, permissions::Permission};
use crate::consts;
#[cfg(feature = "olap")]
use crate::core::errors::{UserErrors, UserResult};

#[allow(dead_code)]
pub struct RoleInfo {
    groups: Vec<PermissionGroup>,
    name: Option<&'static str>,
    is_invitable: bool,
    is_deletable: bool,
    is_updatable: bool,
}

impl RoleInfo {
    pub fn get_permission_groups(&self) -> &Vec<PermissionGroup> {
        &self.groups
    }

    pub fn get_name(&self) -> Option<&'static str> {
        self.name
    }

    pub fn is_invitable(&self) -> bool {
        self.is_invitable
    }

    pub fn check_permission_exists(&self, required_permission: &Permission) -> bool {
        self.groups.iter().any(|module| {
            module
                .get_permissions_groups()
                .contains(required_permission)
        })
    }
}

pub static PREDEFINED_PERMISSIONS: Lazy<HashMap<&'static str, RoleInfo>> = Lazy::new(|| {
    let mut roles = HashMap::new();
    roles.insert(
        consts::user_role::ROLE_ID_INTERNAL_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: None,
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: None,
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
        },
    );

    roles.insert(
        consts::user_role::ROLE_ID_ORGANIZATION_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: Some("Organization Admin"),
            is_invitable: false,
            is_deletable: false,
            is_updatable: false,
        },
    );

    // MERCHANT ROLES
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: Some("Admin"),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_VIEW_ONLY,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: Some("View Only"),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_IAM_ADMIN,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: Some("IAM"),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_DEVELOPER,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: Some("Developer"),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_OPERATOR,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: Some("Operator"),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
        },
    );
    roles.insert(
        consts::user_role::ROLE_ID_MERCHANT_CUSTOMER_SUPPORT,
        RoleInfo {
            groups: vec![
                PermissionGroup::OperationsRead,
                PermissionGroup::OperationsWrite,
            ],
            name: Some("Customer Support"),
            is_invitable: true,
            is_deletable: true,
            is_updatable: true,
        },
    );
    roles
});

pub fn get_role_name_from_id(role_id: &str) -> Option<&'static str> {
    PREDEFINED_PERMISSIONS
        .get(role_id)
        .and_then(|role_info| role_info.name)
}

#[cfg(feature = "olap")]
pub fn is_role_invitable(role_id: &str) -> UserResult<bool> {
    PREDEFINED_PERMISSIONS
        .get(role_id)
        .map(|role_info| role_info.is_invitable)
        .ok_or(UserErrors::InvalidRoleId.into())
        .attach_printable(format!("role_id = {} doesn't exist", role_id))
}

#[cfg(feature = "olap")]
pub fn is_role_deletable(role_id: &str) -> UserResult<bool> {
    PREDEFINED_PERMISSIONS
        .get(role_id)
        .map(|role_info| role_info.is_deletable)
        .ok_or(UserErrors::InvalidRoleId.into())
        .attach_printable(format!("role_id = {} doesn't exist", role_id))
}

#[cfg(feature = "olap")]
pub fn is_role_updatable(role_id: &str) -> UserResult<bool> {
    PREDEFINED_PERMISSIONS
        .get(role_id)
        .map(|role_info| role_info.is_updatable)
        .ok_or(UserErrors::InvalidRoleId.into())
        .attach_printable(format!("role_id = {} doesn't exist", role_id))
}
