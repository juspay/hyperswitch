use common_enums::PermissionGroup;

use super::authentication::AuthToken;
use crate::{
    core::errors::{ApiErrorResponse, RouterResult, StorageErrorExt},
    routes::app::AppStateInfo,
};

#[cfg(feature = "olap")]
pub mod info;
pub mod permission_groups;
pub mod permissions;
pub mod roles;

pub async fn get_permissions<A>(
    state: &A,
    token: &AuthToken,
) -> RouterResult<Vec<permissions::Permission>>
where
    A: AppStateInfo + Sync,
{
    if let Some(role_info) = roles::predefined_roles::PREDEFINED_ROLES.get(token.role_id.as_str()) {
        Ok(get_permissions_from_groups(
            role_info.get_permission_groups(),
        ))
    } else {
        state
            .store()
            .find_role_by_role_id_in_merchant_scope(
                &token.role_id,
                &token.merchant_id,
                &token.org_id,
            )
            .await
            .map(|role| get_permissions_from_groups(&role.groups))
            .to_not_found_response(ApiErrorResponse::InvalidJwtToken)
    }
}

pub fn get_permissions_from_groups(groups: &[PermissionGroup]) -> Vec<permissions::Permission> {
    groups
        .iter()
        .flat_map(|group| {
            permission_groups::get_permissions_vec(group)
                .iter()
                .cloned()
        })
        .collect()
}

pub fn check_authorization(
    required_permission: &permissions::Permission,
    permissions: &[permissions::Permission],
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
