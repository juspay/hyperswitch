use api_models::user_role::role::{self as role_api};
use common_enums::{EntityType, RoleScope};
use common_utils::generate_id_with_default_len;
use diesel_models::role::{RoleNew, RoleUpdate};
use error_stack::{report, ResultExt};

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    routes::{app::ReqState, SessionState},
    services::{
        authentication::{blacklist, UserFromToken},
        authorization::roles::{self, predefined_roles::PREDEFINED_ROLES},
        ApplicationResponse,
    },
    types::domain::user::RoleName,
    utils,
};

pub async fn get_role_from_token_with_permissions(
    state: SessionState,
    user_from_token: UserFromToken,
) -> UserResponse<role_api::GetRoleFromTokenResponse> {
    let role_info = user_from_token
        .get_role_info_from_db(&state)
        .await
        .attach_printable("Invalid role_id in JWT")?;

    let permissions = role_info
        .get_permissions_set()
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(ApplicationResponse::Json(
        role_api::GetRoleFromTokenResponse::Permissions(permissions),
    ))
}

pub async fn get_role_from_token_with_groups(
    state: SessionState,
    user_from_token: UserFromToken,
) -> UserResponse<role_api::GetRoleFromTokenResponse> {
    let role_info = user_from_token
        .get_role_info_from_db(&state)
        .await
        .attach_printable("Invalid role_id in JWT")?;

    let permissions = role_info.get_permission_groups().to_vec();

    Ok(ApplicationResponse::Json(
        role_api::GetRoleFromTokenResponse::Groups(permissions),
    ))
}

pub async fn create_role(
    state: SessionState,
    user_from_token: UserFromToken,
    req: role_api::CreateRoleRequest,
    _req_state: ReqState,
) -> UserResponse<role_api::RoleInfoWithGroupsResponse> {
    let now = common_utils::date_time::now();
    let role_name = RoleName::new(req.role_name)?;

    utils::user_role::validate_role_groups(&req.groups)?;
    utils::user_role::validate_role_name(
        &state,
        &role_name,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await?;

    if matches!(req.role_scope, RoleScope::Organization)
        && user_from_token.role_id != common_utils::consts::ROLE_ID_ORGANIZATION_ADMIN
    {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("Non org admin user creating org level role");
    }

    let role = state
        .store
        .insert_role(RoleNew {
            role_id: generate_id_with_default_len("role"),
            role_name: role_name.get_role_name(),
            merchant_id: user_from_token.merchant_id,
            org_id: user_from_token.org_id,
            groups: req.groups,
            scope: req.role_scope,
            entity_type: Some(EntityType::Merchant),
            created_by: user_from_token.user_id.clone(),
            last_modified_by: user_from_token.user_id,
            created_at: now,
            last_modified_at: now,
        })
        .await
        .to_duplicate_response(UserErrors::RoleNameAlreadyExists)?;

    Ok(ApplicationResponse::Json(
        role_api::RoleInfoWithGroupsResponse {
            groups: role.groups,
            role_id: role.role_id,
            role_name: role.role_name,
            role_scope: role.scope,
        },
    ))
}

// TODO: To be deprecated once groups are stable
pub async fn list_invitable_roles_with_permissions(
    state: SessionState,
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
    state: SessionState,
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
    state: SessionState,
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
    state: SessionState,
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
    state: SessionState,
    user_from_token: UserFromToken,
    req: role_api::UpdateRoleRequest,
    role_id: &str,
) -> UserResponse<role_api::RoleInfoWithGroupsResponse> {
    let role_name = req.role_name.map(RoleName::new).transpose()?;

    if let Some(ref role_name) = role_name {
        utils::user_role::validate_role_name(
            &state,
            role_name,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
        )
        .await?;
    }

    if let Some(ref groups) = req.groups {
        utils::user_role::validate_role_groups(groups)?;
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
        && user_from_token.role_id != common_utils::consts::ROLE_ID_ORGANIZATION_ADMIN
    {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("Non org admin user changing org level role");
    }

    let updated_role = state
        .store
        .update_role_by_role_id(
            role_id,
            RoleUpdate::UpdateDetails {
                groups: req.groups,
                role_name: role_name.map(RoleName::get_role_name),
                last_modified_at: common_utils::date_time::now(),
                last_modified_by: user_from_token.user_id,
            },
        )
        .await
        .to_duplicate_response(UserErrors::RoleNameAlreadyExists)?;

    blacklist::insert_role_in_blacklist(&state, role_id).await?;

    Ok(ApplicationResponse::Json(
        role_api::RoleInfoWithGroupsResponse {
            groups: updated_role.groups,
            role_id: updated_role.role_id,
            role_name: updated_role.role_name,
            role_scope: updated_role.scope,
        },
    ))
}

pub async fn list_roles_with_info(
    state: SessionState,
    user_from_token: UserFromToken,
) -> UserResponse<Vec<role_api::RoleInfoResponseNew>> {
    let user_role_info = user_from_token
        .get_role_info_from_db(&state)
        .await
        .attach_printable("Invalid role_id in JWT")?;

    let predefined_roles_map = PREDEFINED_ROLES
        .iter()
        .filter(|(_, role_info)| user_role_info.get_entity_type() >= role_info.get_entity_type())
        .map(|(role_id, role_info)| role_api::RoleInfoResponseNew {
            role_id: role_id.to_string(),
            role_name: role_info.get_role_name().to_string(),
            entity_type: role_info.get_entity_type(),
            groups: role_info.get_permission_groups().to_vec(),
            scope: role_info.get_scope(),
            merchant_id: None,
        });

    let user_role_entity = user_role_info.get_entity_type();
    let custom_roles = match user_role_entity {
        EntityType::Organization => state
            .store
            .list_roles_for_org_by_parameters(&user_from_token.org_id, None, None, None)
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to get roles")?,
        EntityType::Merchant => state
            .store
            .list_roles_for_org_by_parameters(
                &user_from_token.org_id,
                Some(&user_from_token.merchant_id),
                None,
                None,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to get roles")?,
        // TODO: Populate this from Db function when support for profile id and profile level custom roles is added
        EntityType::Profile => Vec::new(),
        EntityType::Internal => {
            return Err(UserErrors::InvalidRoleOperationWithMessage(
                "Internal roles are not allowed for this operation".to_string(),
            )
            .into());
        }
    };
    let custom_roles_map = custom_roles.into_iter().filter_map(|role| {
        let merchant_id = role.merchant_id.clone();
        let role_info = roles::RoleInfo::from(role);
        (user_role_entity >= role_info.get_entity_type()).then_some(role_api::RoleInfoResponseNew {
            role_id: role_info.get_role_id().to_string(),
            role_name: role_info.get_role_name().to_string(),
            groups: role_info.get_permission_groups().to_vec(),
            entity_type: role_info.get_entity_type(),
            scope: role_info.get_scope(),
            merchant_id: Some(merchant_id),
        })
    });

    Ok(ApplicationResponse::Json(
        predefined_roles_map.chain(custom_roles_map).collect(),
    ))
}

pub async fn list_roles_at_entity_level(
    state: SessionState,
    user_from_token: UserFromToken,
    req: role_api::ListRolesAtEntityLevelRequest,
    check_type: role_api::RoleCheckType,
) -> UserResponse<Vec<role_api::MinimalRoleInfo>> {
    let user_entity_type = user_from_token
        .get_role_info_from_db(&state)
        .await
        .attach_printable("Invalid role_id in JWT")?
        .get_entity_type();

    if req.entity_type > user_entity_type {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User is attempting to request list roles above the current entity level".to_string(),
        )
        .into());
    }

    let predefined_roles_map = PREDEFINED_ROLES
        .iter()
        .filter(|(_, role_info)| {
            let check_type = match check_type {
                role_api::RoleCheckType::Invite => role_info.is_invitable(),
                role_api::RoleCheckType::Update => role_info.is_updatable(),
            };
            check_type && role_info.get_entity_type() == req.entity_type
        })
        .map(|(role_id, role_info)| role_api::MinimalRoleInfo {
            role_id: role_id.to_string(),
            role_name: role_info.get_role_name().to_string(),
        });

    let custom_roles = match req.entity_type {
        EntityType::Organization => state
            .store
            .list_roles_for_org_by_parameters(
                &user_from_token.org_id,
                None,
                Some(req.entity_type),
                None,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to get roles")?,

        EntityType::Merchant => state
            .store
            .list_roles_for_org_by_parameters(
                &user_from_token.org_id,
                Some(&user_from_token.merchant_id),
                Some(req.entity_type),
                None,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to get roles")?,
        // TODO: Populate this from Db function when support for profile id and profile level custom roles is added
        EntityType::Profile => Vec::new(),
        EntityType::Internal => {
            return Err(UserErrors::InvalidRoleOperationWithMessage(
                "Internal roles are not allowed for this operation".to_string(),
            )
            .into());
        }
    };

    let custom_roles_map = custom_roles.into_iter().map(|role| {
        let role_info = roles::RoleInfo::from(role);
        role_api::MinimalRoleInfo {
            role_id: role_info.get_role_id().to_string(),
            role_name: role_info.get_role_name().to_string(),
        }
    });

    Ok(ApplicationResponse::Json(
        predefined_roles_map.chain(custom_roles_map).collect(),
    ))
}
