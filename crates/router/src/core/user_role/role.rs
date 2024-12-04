use std::collections::HashSet;

use api_models::user_role::role as role_api;
use common_enums::{EntityType, ParentGroup, PermissionGroup, RoleScope};
use common_utils::generate_id_with_default_len;
use diesel_models::role::{RoleNew, RoleUpdate};
use error_stack::{report, ResultExt};

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    routes::{app::ReqState, SessionState},
    services::{
        authentication::{blacklist, UserFromToken},
        authorization::{
            permission_groups::{ParentGroupExt, PermissionGroupExt},
            roles::{self, predefined_roles::PREDEFINED_ROLES},
        },
        ApplicationResponse,
    },
    types::domain::user::RoleName,
    utils,
};

pub async fn get_role_from_token_with_groups(
    state: SessionState,
    user_from_token: UserFromToken,
) -> UserResponse<Vec<PermissionGroup>> {
    let role_info = user_from_token
        .get_role_info_from_db(&state)
        .await
        .attach_printable("Invalid role_id in JWT")?;

    let permissions = role_info.get_permission_groups().to_vec();

    Ok(ApplicationResponse::Json(permissions))
}

pub async fn get_groups_and_resources_for_role_from_token(
    state: SessionState,
    user_from_token: UserFromToken,
) -> UserResponse<role_api::GroupsAndResources> {
    let role_info = user_from_token.get_role_info_from_db(&state).await?;

    let groups = role_info
        .get_permission_groups()
        .into_iter()
        .collect::<Vec<_>>();
    let resources = groups
        .iter()
        .flat_map(|group| group.resources())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    Ok(ApplicationResponse::Json(role_api::GroupsAndResources {
        groups,
        resources,
    }))
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
            entity_type: EntityType::Merchant,
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

pub async fn get_role_with_groups(
    state: SessionState,
    user_from_token: UserFromToken,
    role: role_api::GetRoleRequest,
) -> UserResponse<role_api::RoleInfoWithGroupsResponse> {
    let role_info = roles::RoleInfo::from_role_id_in_merchant_scope(
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
        role_api::RoleInfoWithGroupsResponse {
            groups: role_info.get_permission_groups().to_vec(),
            role_id: role.role_id,
            role_name: role_info.get_role_name().to_string(),
            role_scope: role_info.get_scope(),
        },
    ))
}

pub async fn get_parent_info_for_role(
    state: SessionState,
    user_from_token: UserFromToken,
    role: role_api::GetRoleRequest,
) -> UserResponse<role_api::RoleInfoWithParents> {
    let role_info = roles::RoleInfo::from_role_id_in_merchant_scope(
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

    let parent_groups = ParentGroup::get_descriptions_for_groups(
        role_info.get_entity_type(),
        role_info.get_permission_groups().to_vec(),
    )
    .into_iter()
    .map(|(parent_group, description)| role_api::ParentGroupInfo {
        name: parent_group.clone(),
        description,
        scopes: role_info
            .get_permission_groups()
            .iter()
            .filter_map(|group| (group.parent() == parent_group).then_some(group.scope()))
            // TODO: Remove this hashset conversion when merhant access
            // and organization access groups are removed
            .collect::<HashSet<_>>()
            .into_iter()
            .collect(),
    })
    .collect();

    Ok(ApplicationResponse::Json(role_api::RoleInfoWithParents {
        role_id: role.role_id,
        parent_groups,
        role_name: role_info.get_role_name().to_string(),
        role_scope: role_info.get_scope(),
    }))
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

    let role_info = roles::RoleInfo::from_role_id_in_merchant_scope(
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
    request: role_api::ListRolesRequest,
) -> UserResponse<Vec<role_api::RoleInfoResponseNew>> {
    let user_role_info = user_from_token
        .get_role_info_from_db(&state)
        .await
        .attach_printable("Invalid role_id in JWT")?;

    if user_role_info.is_internal() {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "Internal roles are not allowed for this operation".to_string(),
        )
        .into());
    }

    let mut role_info_vec = PREDEFINED_ROLES
        .iter()
        .map(|(_, role_info)| role_info.clone())
        .collect::<Vec<_>>();

    let user_role_entity = user_role_info.get_entity_type();
    let custom_roles =
        match utils::user_role::get_min_entity(user_role_entity, request.entity_type)? {
            EntityType::Tenant | EntityType::Organization => state
                .store
                .list_roles_for_org_by_parameters(
                    &user_from_token.org_id,
                    None,
                    request.entity_type,
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
                    request.entity_type,
                    None,
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to get roles")?,
            // TODO: Populate this from Db function when support for profile id and profile level custom roles is added
            EntityType::Profile => Vec::new(),
        };

    role_info_vec.extend(custom_roles.into_iter().map(roles::RoleInfo::from));

    let list_role_info_response = role_info_vec
        .into_iter()
        .filter_map(|role_info| {
            let is_lower_entity = user_role_entity >= role_info.get_entity_type();
            let request_filter = request.entity_type.map_or(true, |entity_type| {
                entity_type == role_info.get_entity_type()
            });

            (is_lower_entity && request_filter).then_some(role_api::RoleInfoResponseNew {
                role_id: role_info.get_role_id().to_string(),
                role_name: role_info.get_role_name().to_string(),
                groups: role_info.get_permission_groups().to_vec(),
                entity_type: role_info.get_entity_type(),
                scope: role_info.get_scope(),
            })
        })
        .collect::<Vec<_>>();

    Ok(ApplicationResponse::Json(list_role_info_response))
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
    let mut role_info_vec = PREDEFINED_ROLES
        .iter()
        .map(|(_, role_info)| role_info.clone())
        .collect::<Vec<_>>();

    let custom_roles = match req.entity_type {
        EntityType::Tenant | EntityType::Organization => state
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
    };

    role_info_vec.extend(custom_roles.into_iter().map(roles::RoleInfo::from));

    let list_minimal_role_info = role_info_vec
        .into_iter()
        .filter_map(|role_info| {
            let check_type = match check_type {
                role_api::RoleCheckType::Invite => role_info.is_invitable(),
                role_api::RoleCheckType::Update => role_info.is_updatable(),
            };
            if check_type && role_info.get_entity_type() == req.entity_type {
                Some(role_api::MinimalRoleInfo {
                    role_id: role_info.get_role_id().to_string(),
                    role_name: role_info.get_role_name().to_string(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(ApplicationResponse::Json(list_minimal_role_info))
}
