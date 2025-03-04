use std::collections::{HashMap, HashSet};

use api_models::{
    user as user_api,
    user_role::{self as user_role_api, role as role_api},
};
use diesel_models::{
    enums::{UserRoleVersion, UserStatus},
    organization::OrganizationBridge,
    user_role::UserRoleUpdate,
};
use error_stack::{report, ResultExt};
use masking::Secret;
use once_cell::sync::Lazy;

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    db::user_role::{ListUserRolesByOrgIdPayload, ListUserRolesByUserIdPayload},
    routes::{app::ReqState, SessionState},
    services::{
        authentication as auth,
        authorization::{
            info,
            permission_groups::{ParentGroupExt, PermissionGroupExt},
            roles,
        },
        ApplicationResponse,
    },
    types::domain,
    utils,
};
pub mod role;
use common_enums::{EntityType, ParentGroup, PermissionGroup};
use strum::IntoEnumIterator;

// TODO: To be deprecated
pub async fn get_authorization_info_with_groups(
    _state: SessionState,
) -> UserResponse<user_role_api::AuthorizationInfoResponse> {
    Ok(ApplicationResponse::Json(
        user_role_api::AuthorizationInfoResponse(
            info::get_group_authorization_info()
                .into_iter()
                .map(user_role_api::AuthorizationInfo::Group)
                .collect(),
        ),
    ))
}

pub async fn get_authorization_info_with_group_tag(
) -> UserResponse<user_role_api::AuthorizationInfoResponse> {
    static GROUPS_WITH_PARENT_TAGS: Lazy<Vec<user_role_api::ParentInfo>> = Lazy::new(|| {
        PermissionGroup::iter()
            .map(|group| (group.parent(), group))
            .fold(
                HashMap::new(),
                |mut acc: HashMap<ParentGroup, Vec<PermissionGroup>>, (key, value)| {
                    acc.entry(key).or_default().push(value);
                    acc
                },
            )
            .into_iter()
            .map(|(name, value)| user_role_api::ParentInfo {
                name: name.clone(),
                description: info::get_parent_group_description(name),
                groups: value,
            })
            .collect()
    });

    Ok(ApplicationResponse::Json(
        user_role_api::AuthorizationInfoResponse(
            GROUPS_WITH_PARENT_TAGS
                .iter()
                .cloned()
                .map(user_role_api::AuthorizationInfo::GroupWithTag)
                .collect(),
        ),
    ))
}

pub async fn get_parent_group_info(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<Vec<role_api::ParentGroupInfo>> {
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    let parent_groups = ParentGroup::get_descriptions_for_groups(
        role_info.get_entity_type(),
        PermissionGroup::iter().collect(),
    )
    .into_iter()
    .map(|(parent_group, description)| role_api::ParentGroupInfo {
        name: parent_group.clone(),
        description,
        scopes: PermissionGroup::iter()
            .filter_map(|group| (group.parent() == parent_group).then_some(group.scope()))
            // TODO: Remove this hashset conversion when merhant access
            // and organization access groups are removed
            .collect::<HashSet<_>>()
            .into_iter()
            .collect(),
    })
    .collect::<Vec<_>>();

    Ok(ApplicationResponse::Json(parent_groups))
}

pub async fn update_user_role(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: user_role_api::UpdateUserRoleRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let role_info = roles::RoleInfo::from_role_id_in_lineage(
        &state,
        &req.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
        &user_from_token.profile_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    if !role_info.is_updatable() {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable(format!("User role cannot be updated to {}", req.role_id));
    }

    let user_to_be_updated =
        utils::user::get_user_from_db_by_email(&state, domain::UserEmail::try_from(req.email)?)
            .await
            .to_not_found_response(UserErrors::InvalidRoleOperation)
            .attach_printable("User not found in our records".to_string())?;

    if user_from_token.user_id == user_to_be_updated.get_user_id() {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("User Changing their own role");
    }

    let updator_role = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let mut is_updated = false;

    let v2_user_role_to_be_updated = match state
        .global_store
        .find_user_role_by_user_id_and_lineage(
            user_to_be_updated.get_user_id(),
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            &user_from_token.profile_id,
            UserRoleVersion::V2,
        )
        .await
    {
        Ok(user_role) => Some(user_role),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                None
            } else {
                return Err(UserErrors::InternalServerError.into());
            }
        }
    };

    if let Some(user_role) = v2_user_role_to_be_updated {
        let role_to_be_updated = roles::RoleInfo::from_role_id_org_id_tenant_id(
            &state,
            &user_role.role_id,
            &user_from_token.org_id,
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

        if !role_to_be_updated.is_updatable() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "User role cannot be updated from {}",
                role_to_be_updated.get_role_id()
            ));
        }

        if role_info.get_entity_type() != role_to_be_updated.get_entity_type() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "Upgrade and downgrade of roles is not allowed, user_entity_type = {} req_entity_type = {}",
                role_to_be_updated.get_entity_type(),
                role_info.get_entity_type(),
            ));
        }

        if updator_role.get_entity_type() < role_to_be_updated.get_entity_type() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "Invalid operation, update requestor = {} cannot update target = {}",
                updator_role.get_entity_type(),
                role_to_be_updated.get_entity_type()
            ));
        }

        state
            .global_store
            .update_user_role_by_user_id_and_lineage(
                user_to_be_updated.get_user_id(),
                user_from_token
                    .tenant_id
                    .as_ref()
                    .unwrap_or(&state.tenant.tenant_id),
                &user_from_token.org_id,
                Some(&user_from_token.merchant_id),
                Some(&user_from_token.profile_id),
                UserRoleUpdate::UpdateRole {
                    role_id: req.role_id.clone(),
                    modified_by: user_from_token.user_id.clone(),
                },
                UserRoleVersion::V2,
            )
            .await
            .change_context(UserErrors::InternalServerError)?;

        is_updated = true;
    }

    let v1_user_role_to_be_updated = match state
        .global_store
        .find_user_role_by_user_id_and_lineage(
            user_to_be_updated.get_user_id(),
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            &user_from_token.profile_id,
            UserRoleVersion::V1,
        )
        .await
    {
        Ok(user_role) => Some(user_role),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                None
            } else {
                return Err(UserErrors::InternalServerError.into());
            }
        }
    };

    if let Some(user_role) = v1_user_role_to_be_updated {
        let role_to_be_updated = roles::RoleInfo::from_role_id_org_id_tenant_id(
            &state,
            &user_role.role_id,
            &user_from_token.org_id,
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

        if !role_to_be_updated.is_updatable() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "User role cannot be updated from {}",
                role_to_be_updated.get_role_id()
            ));
        }

        if role_info.get_entity_type() != role_to_be_updated.get_entity_type() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "Upgrade and downgrade of roles is not allowed, user_entity_type = {} req_entity_type = {}",
                role_to_be_updated.get_entity_type(),
                role_info.get_entity_type(),
            ));
        }

        if updator_role.get_entity_type() < role_to_be_updated.get_entity_type() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "Invalid operation, update requestor = {} cannot update target = {}",
                updator_role.get_entity_type(),
                role_to_be_updated.get_entity_type()
            ));
        }

        state
            .global_store
            .update_user_role_by_user_id_and_lineage(
                user_to_be_updated.get_user_id(),
                user_from_token
                    .tenant_id
                    .as_ref()
                    .unwrap_or(&state.tenant.tenant_id),
                &user_from_token.org_id,
                Some(&user_from_token.merchant_id),
                Some(&user_from_token.profile_id),
                UserRoleUpdate::UpdateRole {
                    role_id: req.role_id.clone(),
                    modified_by: user_from_token.user_id,
                },
                UserRoleVersion::V1,
            )
            .await
            .change_context(UserErrors::InternalServerError)?;

        is_updated = true;
    }

    if !is_updated {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("User with given email is not found in the organization")?;
    }

    auth::blacklist::insert_user_in_blacklist(&state, user_to_be_updated.get_user_id()).await?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn accept_invitations_v2(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: user_role_api::AcceptInvitationsV2Request,
) -> UserResponse<()> {
    let lineages = futures::future::try_join_all(req.into_iter().map(|entity| {
        utils::user_role::get_lineage_for_user_id_and_entity_for_accepting_invite(
            &state,
            &user_from_token.user_id,
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            entity.entity_id,
            entity.entity_type,
        )
    }))
    .await?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let update_results = futures::future::join_all(lineages.iter().map(
        |(org_id, merchant_id, profile_id)| async {
            let (update_v1_result, update_v2_result) =
                utils::user_role::update_v1_and_v2_user_roles_in_db(
                    &state,
                    user_from_token.user_id.as_str(),
                    user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id,
                    merchant_id.as_ref(),
                    profile_id.as_ref(),
                    UserRoleUpdate::UpdateStatus {
                        status: UserStatus::Active,
                        modified_by: user_from_token.user_id.clone(),
                    },
                )
                .await;

            if update_v1_result.is_err_and(|err| !err.current_context().is_db_not_found())
                || update_v2_result.is_err_and(|err| !err.current_context().is_db_not_found())
            {
                Err(report!(UserErrors::InternalServerError))
            } else {
                Ok(())
            }
        },
    ))
    .await;

    if update_results.is_empty() || update_results.iter().all(Result::is_err) {
        return Err(UserErrors::MerchantIdNotFound.into());
    }

    Ok(ApplicationResponse::StatusOk)
}

pub async fn accept_invitations_pre_auth(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_role_api::AcceptInvitationsPreAuthRequest,
) -> UserResponse<user_api::TokenResponse> {
    let lineages = futures::future::try_join_all(req.into_iter().map(|entity| {
        utils::user_role::get_lineage_for_user_id_and_entity_for_accepting_invite(
            &state,
            &user_token.user_id,
            user_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            entity.entity_id,
            entity.entity_type,
        )
    }))
    .await?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let update_results = futures::future::join_all(lineages.iter().map(
        |(org_id, merchant_id, profile_id)| async {
            let (update_v1_result, update_v2_result) =
                utils::user_role::update_v1_and_v2_user_roles_in_db(
                    &state,
                    user_token.user_id.as_str(),
                    user_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id,
                    merchant_id.as_ref(),
                    profile_id.as_ref(),
                    UserRoleUpdate::UpdateStatus {
                        status: UserStatus::Active,
                        modified_by: user_token.user_id.clone(),
                    },
                )
                .await;

            if update_v1_result.is_err_and(|err| !err.current_context().is_db_not_found())
                || update_v2_result.is_err_and(|err| !err.current_context().is_db_not_found())
            {
                Err(report!(UserErrors::InternalServerError))
            } else {
                Ok(())
            }
        },
    ))
    .await;

    if update_results.is_empty() || update_results.iter().all(Result::is_err) {
        return Err(UserErrors::MerchantIdNotFound.into());
    }

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(user_token.user_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let current_flow =
        domain::CurrentFlow::new(user_token, domain::SPTFlow::MerchantSelect.into())?;
    let next_flow = current_flow.next(user_from_db.clone(), &state).await?;

    let token = next_flow.get_token(&state).await?;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    };
    auth::cookies::set_cookie_response(response, token)
}

pub async fn delete_user_role(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: user_role_api::DeleteUserRoleRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(&domain::UserEmail::from_pii_email(request.email)?)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::InvalidRoleOperation)
                    .attach_printable("User not found in our records")
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?
        .into();

    if user_from_db.get_user_id() == user_from_token.user_id {
        return Err(report!(UserErrors::InvalidDeleteOperation))
            .attach_printable("User deleting himself");
    }

    let deletion_requestor_role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let mut user_role_deleted_flag = false;

    // Find in V2
    let user_role_v2 = match state
        .global_store
        .find_user_role_by_user_id_and_lineage(
            user_from_db.get_user_id(),
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            &user_from_token.profile_id,
            UserRoleVersion::V2,
        )
        .await
    {
        Ok(user_role) => Some(user_role),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                None
            } else {
                return Err(UserErrors::InternalServerError.into());
            }
        }
    };

    if let Some(role_to_be_deleted) = user_role_v2 {
        let target_role_info = roles::RoleInfo::from_role_id_in_lineage(
            &state,
            &role_to_be_deleted.role_id,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
            &user_from_token.profile_id,
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

        if !target_role_info.is_deletable() {
            return Err(report!(UserErrors::InvalidDeleteOperation)).attach_printable(format!(
                "Invalid operation, role_id = {} is not deletable",
                role_to_be_deleted.role_id
            ));
        }

        if deletion_requestor_role_info.get_entity_type() < target_role_info.get_entity_type() {
            return Err(report!(UserErrors::InvalidDeleteOperation)).attach_printable(format!(
                "Invalid operation, deletion requestor = {} cannot delete target = {}",
                deletion_requestor_role_info.get_entity_type(),
                target_role_info.get_entity_type()
            ));
        }

        user_role_deleted_flag = true;
        state
            .global_store
            .delete_user_role_by_user_id_and_lineage(
                user_from_db.get_user_id(),
                user_from_token
                    .tenant_id
                    .as_ref()
                    .unwrap_or(&state.tenant.tenant_id),
                &user_from_token.org_id,
                &user_from_token.merchant_id,
                &user_from_token.profile_id,
                UserRoleVersion::V2,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user role")?;
    }

    // Find in V1
    let user_role_v1 = match state
        .global_store
        .find_user_role_by_user_id_and_lineage(
            user_from_db.get_user_id(),
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            &user_from_token.profile_id,
            UserRoleVersion::V1,
        )
        .await
    {
        Ok(user_role) => Some(user_role),
        Err(e) => {
            if e.current_context().is_db_not_found() {
                None
            } else {
                return Err(UserErrors::InternalServerError.into());
            }
        }
    };

    if let Some(role_to_be_deleted) = user_role_v1 {
        let target_role_info = roles::RoleInfo::from_role_id_in_lineage(
            &state,
            &role_to_be_deleted.role_id,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
            &user_from_token.profile_id,
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

        if !target_role_info.is_deletable() {
            return Err(report!(UserErrors::InvalidDeleteOperation)).attach_printable(format!(
                "Invalid operation, role_id = {} is not deletable",
                role_to_be_deleted.role_id
            ));
        }

        if deletion_requestor_role_info.get_entity_type() < target_role_info.get_entity_type() {
            return Err(report!(UserErrors::InvalidDeleteOperation)).attach_printable(format!(
                "Invalid operation, deletion requestor = {} cannot delete target = {}",
                deletion_requestor_role_info.get_entity_type(),
                target_role_info.get_entity_type()
            ));
        }

        user_role_deleted_flag = true;
        state
            .global_store
            .delete_user_role_by_user_id_and_lineage(
                user_from_db.get_user_id(),
                user_from_token
                    .tenant_id
                    .as_ref()
                    .unwrap_or(&state.tenant.tenant_id),
                &user_from_token.org_id,
                &user_from_token.merchant_id,
                &user_from_token.profile_id,
                UserRoleVersion::V1,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user role")?;
    }

    if !user_role_deleted_flag {
        return Err(report!(UserErrors::InvalidDeleteOperation))
            .attach_printable("User is not associated with the merchant");
    }

    // Check if user has any more role associations
    let remaining_roles = state
        .global_store
        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
            user_id: user_from_db.get_user_id(),
            tenant_id: user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),

            org_id: None,
            merchant_id: None,
            profile_id: None,
            entity_id: None,
            version: None,
            status: None,
            limit: None,
        })
        .await
        .change_context(UserErrors::InternalServerError)?;

    // If user has no more role associated with him then deleting user
    if remaining_roles.is_empty() {
        state
            .global_store
            .delete_user_by_user_id(user_from_db.get_user_id())
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user entry")?;
    }

    auth::blacklist::insert_user_in_blacklist(&state, user_from_db.get_user_id()).await?;
    Ok(ApplicationResponse::StatusOk)
}

pub async fn list_users_in_lineage(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: user_role_api::ListUsersInEntityRequest,
) -> UserResponse<Vec<user_role_api::ListUsersInEntityResponse>> {
    let requestor_role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let user_roles_set: HashSet<_> = match utils::user_role::get_min_entity(
        requestor_role_info.get_entity_type(),
        request.entity_type,
    )? {
        EntityType::Tenant => {
            let mut org_users = utils::user_role::fetch_user_roles_by_payload(
                &state,
                ListUserRolesByOrgIdPayload {
                    user_id: None,
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: &user_from_token.org_id,
                    merchant_id: None,
                    profile_id: None,
                    version: None,
                    limit: None,
                },
                request.entity_type,
            )
            .await?;

            // Fetch tenant user
            let tenant_user = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id: &user_from_token.user_id,
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: None,
                    merchant_id: None,
                    profile_id: None,
                    entity_id: None,
                    version: None,
                    status: None,
                    limit: None,
                })
                .await
                .change_context(UserErrors::InternalServerError)?;

            org_users.extend(tenant_user);
            org_users
        }
        EntityType::Organization => {
            utils::user_role::fetch_user_roles_by_payload(
                &state,
                ListUserRolesByOrgIdPayload {
                    user_id: None,
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: &user_from_token.org_id,
                    merchant_id: None,
                    profile_id: None,
                    version: None,
                    limit: None,
                },
                request.entity_type,
            )
            .await?
        }
        EntityType::Merchant => {
            utils::user_role::fetch_user_roles_by_payload(
                &state,
                ListUserRolesByOrgIdPayload {
                    user_id: None,
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: &user_from_token.org_id,
                    merchant_id: Some(&user_from_token.merchant_id),
                    profile_id: None,
                    version: None,
                    limit: None,
                },
                request.entity_type,
            )
            .await?
        }
        EntityType::Profile => {
            utils::user_role::fetch_user_roles_by_payload(
                &state,
                ListUserRolesByOrgIdPayload {
                    user_id: None,
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: &user_from_token.org_id,
                    merchant_id: Some(&user_from_token.merchant_id),
                    profile_id: Some(&user_from_token.profile_id),
                    version: None,
                    limit: None,
                },
                request.entity_type,
            )
            .await?
        }
    };

    // This filtering is needed because for org level users in V1, merchant_id is present.
    // Due to this, we get org level users in merchant level users list.
    let user_roles_set = user_roles_set
        .into_iter()
        .filter_map(|user_role| {
            let (_entity_id, entity_type) = user_role.get_entity_id_and_type()?;
            (entity_type <= requestor_role_info.get_entity_type()).then_some(user_role)
        })
        .collect::<HashSet<_>>();

    let mut email_map = state
        .global_store
        .find_users_by_user_ids(
            user_roles_set
                .iter()
                .map(|user_role| user_role.user_id.clone())
                .collect(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .map(|user| (user.user_id.clone(), user.email))
        .collect::<HashMap<_, _>>();

    let role_info_map =
        futures::future::try_join_all(user_roles_set.iter().map(|user_role| async {
            roles::RoleInfo::from_role_id_org_id_tenant_id(
                &state,
                &user_role.role_id,
                &user_from_token.org_id,
                user_from_token
                    .tenant_id
                    .as_ref()
                    .unwrap_or(&state.tenant.tenant_id),
            )
            .await
            .map(|role_info| {
                (
                    user_role.role_id.clone(),
                    user_role_api::role::MinimalRoleInfo {
                        role_id: user_role.role_id.clone(),
                        role_name: role_info.get_role_name().to_string(),
                    },
                )
            })
        }))
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .collect::<HashMap<_, _>>();

    let user_role_map = user_roles_set
        .into_iter()
        .fold(HashMap::new(), |mut map, user_role| {
            map.entry(user_role.user_id)
                .or_insert(Vec::with_capacity(1))
                .push(user_role.role_id);
            map
        });

    Ok(ApplicationResponse::Json(
        user_role_map
            .into_iter()
            .map(|(user_id, role_id_vec)| {
                Ok::<_, error_stack::Report<UserErrors>>(user_role_api::ListUsersInEntityResponse {
                    email: email_map
                        .remove(&user_id)
                        .ok_or(UserErrors::InternalServerError)?,
                    roles: role_id_vec
                        .into_iter()
                        .map(|role_id| {
                            role_info_map
                                .get(&role_id)
                                .cloned()
                                .ok_or(UserErrors::InternalServerError)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

pub async fn list_invitations_for_user(
    state: SessionState,
    user_from_token: auth::UserIdFromAuth,
) -> UserResponse<Vec<user_role_api::ListInvitationForUserResponse>> {
    let user_roles = state
        .global_store
        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
            user_id: &user_from_token.user_id,
            tenant_id: user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            org_id: None,
            merchant_id: None,
            profile_id: None,
            entity_id: None,
            version: None,
            status: Some(UserStatus::InvitationSent),
            limit: None,
        })
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to list user roles by user id and invitation sent")?
        .into_iter()
        .collect::<HashSet<_>>();

    let (org_ids, merchant_ids, profile_ids_with_merchant_ids) = user_roles.iter().try_fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut org_ids, mut merchant_ids, mut profile_ids_with_merchant_ids), user_role| {
            let (_, entity_type) = user_role
                .get_entity_id_and_type()
                .ok_or(UserErrors::InternalServerError)
                .attach_printable("Failed to compute entity id and type")?;

            match entity_type {
                EntityType::Tenant => {
                    return Err(report!(UserErrors::InternalServerError))
                        .attach_printable("Tenant roles are not allowed for this operation");
                }
                EntityType::Organization => org_ids.push(
                    user_role
                        .org_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)?,
                ),
                EntityType::Merchant => merchant_ids.push(
                    user_role
                        .merchant_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)?,
                ),
                EntityType::Profile => profile_ids_with_merchant_ids.push((
                    user_role
                        .profile_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)?,
                    user_role
                        .merchant_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)?,
                )),
            }

            Ok::<_, error_stack::Report<UserErrors>>((
                org_ids,
                merchant_ids,
                profile_ids_with_merchant_ids,
            ))
        },
    )?;

    let org_name_map = futures::future::try_join_all(org_ids.into_iter().map(|org_id| async {
        let org_name = state
            .accounts_store
            .find_organization_by_org_id(&org_id)
            .await
            .change_context(UserErrors::InternalServerError)?
            .get_organization_name()
            .map(Secret::new);

        Ok::<_, error_stack::Report<UserErrors>>((org_id, org_name))
    }))
    .await?
    .into_iter()
    .collect::<HashMap<_, _>>();

    let key_manager_state = &(&state).into();

    let merchant_name_map = state
        .store
        .list_multiple_merchant_accounts(key_manager_state, merchant_ids)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into_iter()
        .map(|merchant| {
            (
                merchant.get_id().clone(),
                merchant
                    .merchant_name
                    .map(|encryptable_name| encryptable_name.into_inner()),
            )
        })
        .collect::<HashMap<_, _>>();

    let master_key = &state.store.get_master_key().to_vec().into();

    let profile_name_map = futures::future::try_join_all(profile_ids_with_merchant_ids.iter().map(
        |(profile_id, merchant_id)| async {
            let merchant_key_store = state
                .store
                .get_merchant_key_store_by_merchant_id(key_manager_state, merchant_id, master_key)
                .await
                .change_context(UserErrors::InternalServerError)?;

            let business_profile = state
                .store
                .find_business_profile_by_profile_id(
                    key_manager_state,
                    &merchant_key_store,
                    profile_id,
                )
                .await
                .change_context(UserErrors::InternalServerError)?;

            Ok::<_, error_stack::Report<UserErrors>>((
                profile_id.clone(),
                Secret::new(business_profile.profile_name),
            ))
        },
    ))
    .await?
    .into_iter()
    .collect::<HashMap<_, _>>();

    user_roles
        .into_iter()
        .map(|user_role| {
            let (entity_id, entity_type) = user_role
                .get_entity_id_and_type()
                .ok_or(UserErrors::InternalServerError)
                .attach_printable("Failed to compute entity id and type")?;

            let entity_name = match entity_type {
                EntityType::Tenant => {
                    return Err(report!(UserErrors::InternalServerError))
                        .attach_printable("Tenant roles are not allowed for this operation");
                }
                EntityType::Organization => user_role
                    .org_id
                    .as_ref()
                    .and_then(|org_id| org_name_map.get(org_id).cloned())
                    .ok_or(UserErrors::InternalServerError)?,
                EntityType::Merchant => user_role
                    .merchant_id
                    .as_ref()
                    .and_then(|merchant_id| merchant_name_map.get(merchant_id).cloned())
                    .ok_or(UserErrors::InternalServerError)?,
                EntityType::Profile => user_role
                    .profile_id
                    .as_ref()
                    .map(|profile_id| profile_name_map.get(profile_id).cloned())
                    .ok_or(UserErrors::InternalServerError)?,
            };

            Ok(user_role_api::ListInvitationForUserResponse {
                entity_id,
                entity_type,
                entity_name,
                role_id: user_role.role_id,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(ApplicationResponse::Json)
}
