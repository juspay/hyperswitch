use api_models::user_role::{
    role::{self as role_api},
    Permission,
};
use common_enums::RoleScope;
use common_utils::generate_id_with_default_len;
use diesel_models::role::{RoleNew, RoleUpdate};
use error_stack::ResultExt;

use crate::{
    consts,
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    routes::AppState,
    services::{
        authentication::{blacklist, UserFromToken},
        authorization::roles::{self, predefined_roles::PREDEFINED_ROLES},
        ApplicationResponse,
    },
    types::domain::user::RoleName,
    utils,
};

pub async fn get_role_from_token(
    state: AppState,
    user_from_token: UserFromToken,
) -> UserResponse<Vec<Permission>> {
    let role_info = user_from_token
        .get_role_info_from_db(&state)
        .await
        .attach_printable("Invalid role_id in JWT")?;

    let permissions = role_info
        .get_permissions_set()
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(ApplicationResponse::Json(permissions))
}

pub async fn create_role(
    state: AppState,
    user_from_token: UserFromToken,
    req: role_api::CreateRoleRequest,
) -> UserResponse<()> {
    let now = common_utils::date_time::now();
    let role_name = RoleName::new(req.role_name)?.get_role_name();

    if req.groups.is_empty() {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("Role groups cannot be empty");
    }

    if matches!(req.role_scope, RoleScope::Organization)
        && user_from_token.role_id != consts::user_role::ROLE_ID_ORGANIZATION_ADMIN
    {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("Non org admin user creating org level role");
    }

    utils::user_role::is_role_name_already_present_for_merchant(
        &state,
        &role_name,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await?;

    state
        .store
        .insert_role(RoleNew {
            role_id: generate_id_with_default_len("role"),
            role_name,
            merchant_id: user_from_token.merchant_id,
            org_id: user_from_token.org_id,
            groups: req.groups,
            scope: req.role_scope,
            created_by: user_from_token.user_id.clone(),
            last_modified_by: user_from_token.user_id,
            created_at: now,
            last_modified_at: now,
        })
        .await
        .to_duplicate_response(UserErrors::RoleNameAlreadyExists)?;

    Ok(ApplicationResponse::StatusOk)
}

// TODO: To be deprecated once groups are stable
pub async fn list_invitable_roles_with_permissions(
    state: AppState,
    user_from_token: UserFromToken,
) -> UserResponse<role_api::ListRolesResponse> {
    let predefined_roles_map = PREDEFINED_ROLES
        .iter()
        .filter(|(_, role_info)| role_info.is_invitable())
        .map(|(role_id, role_info)| {
            role_api::RoleInfoResponse::Permissions(role_api::RoleInfoWithPermissionsResponse {
                permissions: role_info
                    .get_permissions_set()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
                role_id: role_id.to_string(),
                role_name: role_info.get_role_name().to_string(),
                role_scope: role_info.get_scope(),
            })
        });

    let custom_roles_map = state
        .store
        .list_all_roles(&user_from_token.merchant_id, &user_from_token.org_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .filter_map(|role| {
            let role_info = roles::RoleInfo::from(role);
            role_info
                .is_invitable()
                .then_some(role_api::RoleInfoResponse::Permissions(
                    role_api::RoleInfoWithPermissionsResponse {
                        permissions: role_info
                            .get_permissions_set()
                            .into_iter()
                            .map(Into::into)
                            .collect(),
                        role_id: role_info.get_role_id().to_string(),
                        role_name: role_info.get_role_name().to_string(),
                        role_scope: role_info.get_scope(),
                    },
                ))
        });

    Ok(ApplicationResponse::Json(role_api::ListRolesResponse(
        predefined_roles_map.chain(custom_roles_map).collect(),
    )))
}

pub async fn list_invitable_roles_with_groups(
    state: AppState,
    user_from_token: UserFromToken,
) -> UserResponse<role_api::ListRolesResponse> {
    let predefined_roles_map = PREDEFINED_ROLES
        .iter()
        .filter(|(_, role_info)| role_info.is_invitable())
        .map(|(role_id, role_info)| {
            role_api::RoleInfoResponse::Groups(role_api::RoleInfoWithGroupsResponse {
                groups: role_info.get_permission_groups().to_vec(),
                role_id: role_id.to_string(),
                role_name: role_info.get_role_name().to_string(),
                role_scope: role_info.get_scope(),
            })
        });

    let custom_roles_map = state
        .store
        .list_all_roles(&user_from_token.merchant_id, &user_from_token.org_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .filter_map(|role| {
            let role_info = roles::RoleInfo::from(role);
            role_info
                .is_invitable()
                .then_some(role_api::RoleInfoResponse::Groups(
                    role_api::RoleInfoWithGroupsResponse {
                        groups: role_info.get_permission_groups().to_vec(),
                        role_id: role_info.get_role_id().to_string(),
                        role_name: role_info.get_role_name().to_string(),
                        role_scope: role_info.get_scope(),
                    },
                ))
        });

    Ok(ApplicationResponse::Json(role_api::ListRolesResponse(
        predefined_roles_map.chain(custom_roles_map).collect(),
    )))
}

// TODO: To be deprecated once groups are stable
pub async fn get_role_with_permissions(
    state: AppState,
    user_from_token: UserFromToken,
    role: role_api::GetRoleRequest,
) -> UserResponse<role_api::RoleInfoResponse> {
    let role_info = roles::RoleInfo::from_role_id(
        &state,
        &role.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    if role_info.is_internal() {
        return Err(UserErrors::InvalidRoleId.into());
    }

    let permissions = role_info
        .get_permissions_set()
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(ApplicationResponse::Json(
        role_api::RoleInfoResponse::Permissions(role_api::RoleInfoWithPermissionsResponse {
            permissions,
            role_id: role.role_id,
            role_name: role_info.get_role_name().to_string(),
            role_scope: role_info.get_scope(),
        }),
    ))
}

pub async fn get_role_with_groups(
    state: AppState,
    user_from_token: UserFromToken,
    role: role_api::GetRoleRequest,
) -> UserResponse<role_api::RoleInfoResponse> {
    let role_info = roles::RoleInfo::from_role_id(
        &state,
        &role.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    if role_info.is_internal() {
        return Err(UserErrors::InvalidRoleId.into());
    }

    Ok(ApplicationResponse::Json(
        role_api::RoleInfoResponse::Groups(role_api::RoleInfoWithGroupsResponse {
            groups: role_info.get_permission_groups().to_vec(),
            role_id: role.role_id,
            role_name: role_info.get_role_name().to_string(),
            role_scope: role_info.get_scope(),
        }),
    ))
}

pub async fn update_role(
    state: AppState,
    user_from_token: UserFromToken,
    req: role_api::UpdateRoleRequest,
    role_id: &str,
) -> UserResponse<()> {
    let role_name = req
        .role_name
        .map(RoleName::new)
        .transpose()?
        .map(RoleName::get_role_name);

    if let Some(ref role_name) = role_name {
        utils::user_role::is_role_name_already_present_for_merchant(
            &state,
            role_name,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
        )
        .await?;
    }

    let role_info = roles::RoleInfo::from_role_id(
        &state,
        role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleOperation)?;

    if matches!(role_info.get_scope(), RoleScope::Organization)
        && user_from_token.role_id != consts::user_role::ROLE_ID_ORGANIZATION_ADMIN
    {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("Non org admin user creating org level role");
    }

    if let Some(ref groups) = req.groups {
        if groups.is_empty() {
            return Err(UserErrors::InvalidRoleOperation.into())
                .attach_printable("role groups cannot be empty");
        }
    }

    state
        .store
        .update_role_by_role_id(
            role_id,
            RoleUpdate::UpdateDetails {
                groups: req.groups,
                role_name,
                last_modified_at: common_utils::date_time::now(),
                last_modified_by: user_from_token.user_id,
            },
        )
        .await
        .to_duplicate_response(UserErrors::RoleNameAlreadyExists)?;

    blacklist::insert_role_in_blacklist(&state, role_id).await?;

    Ok(ApplicationResponse::StatusOk)
}
