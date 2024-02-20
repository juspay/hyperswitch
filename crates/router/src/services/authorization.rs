use common_enums::PermissionGroup;
use crate::{
    core::errors::{ApiErrorResponse, RouterResult, StorageErrorExt},
    routes::app::AppStateInfo,
};

pub mod info;
pub mod permission_groups;
pub mod permissions;
pub mod roles;

pub async fn get_permissions<A>(
    state: &A,
    role_id: &str,
) -> RouterResult<Vec<permissions::Permission>>
where
    A: AppStateInfo + Sync,
{
    if let Some(role_info) = roles::predefined_roles::PREDEFINED_ROLES.get(role_id) {
        Ok(get_permissions_from_groups(
            role_info.get_permission_groups(),
        ))
    } else {
        let role = state
            .store()
            .find_role_by_role_id(role_id)
            .await
            .to_not_found_response(ApiErrorResponse::InvalidJwtToken)?;
        Ok(get_permissions_from_groups(
            &role.groups.into_iter().map(|x| x.into()).collect(),
        ))
    }
}

pub fn get_permissions_from_groups(groups: &Vec<PermissionGroup>) -> Vec<permissions::Permission> {
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
    permissions: Vec<permissions::Permission>,
) -> RouterResult<()> {
    permissions
        .contains(&required_permission)
        .then_some(())
        .ok_or(
            ApiErrorResponse::AccessForbidden {
                resource: required_permission.to_string(),
            }
            .into(),
        )
}
