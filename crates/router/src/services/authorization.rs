use std::collections::HashSet;

use crate::core::errors::{ApiErrorResponse, RouterResult};

pub mod info;
pub mod permission_groups;
pub mod permissions;
pub mod predefined_permissions;

pub fn get_role_info(role: &str) -> RouterResult<&predefined_permissions::RoleInfo> {
    if let Some(role_info) = predefined_permissions::PREDEFINED_PERMISSIONS.get(role) {
        Ok(role_info)
    } else {
        // get role info from db
        todo!()
    }
}

pub fn check_authorization(
    required_permission: &permissions::Permission,
    permissions: &HashSet<permissions::Permission>,
) -> RouterResult<()> {
    permissions
        .contains(required_permission)
        .then_some(())
        .ok_or(
            ApiErrorResponse::AccessForbidden {
                resource: required_permission.to_string(),
            }
            .into(),
        )
}
