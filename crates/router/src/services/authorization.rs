use crate::core::errors::{ApiErrorResponse, RouterResult};

pub mod info;
pub mod permissions;
pub mod predefined_permissions;

pub fn get_permissions(role: &str) -> RouterResult<&Vec<permissions::Permission>> {
    predefined_permissions::PREDEFINED_PERMISSIONS
        .get(role)
        .map(|role_info| role_info.get_permissions())
        .ok_or(ApiErrorResponse::InvalidJwtToken.into())
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
