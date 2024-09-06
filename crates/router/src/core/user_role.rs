use std::collections::{HashMap, HashSet};

use api_models::{user as user_api, user_role as user_role_api};
use diesel_models::{
    enums::{UserRoleVersion, UserStatus},
    user_role::UserRoleUpdate,
};
use error_stack::{report, ResultExt};
use once_cell::sync::Lazy;

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    db::user_role::{ListUserRolesByOrgIdPayload, ListUserRolesByUserIdPayload},
    routes::{app::ReqState, SessionState},
    services::{
        authentication as auth,
        authorization::{info, roles},
        ApplicationResponse,
    },
    types::domain,
    utils,
};
pub mod role;
use common_enums::{EntityType, PermissionGroup};
use strum::IntoEnumIterator;

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
            .map(|value| (info::get_parent_name(value), value))
            .fold(
                HashMap::new(),
                |mut acc: HashMap<user_role_api::ParentGroup, Vec<PermissionGroup>>,
                 (key, value)| {
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

pub async fn update_user_role(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: user_role_api::UpdateUserRoleRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let role_info = roles::RoleInfo::from_role_id(
        &state,
        &req.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
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

    let updator_role = roles::RoleInfo::from_role_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let mut is_updated = false;

    let v2_user_role_to_be_updated = match state
        .store
        .find_user_role_by_user_id_and_lineage(
            user_to_be_updated.get_user_id(),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            None,
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
        let role_to_be_updated = roles::RoleInfo::from_role_id(
            &state,
            &user_role.role_id,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

        if !role_to_be_updated.is_updatable() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "User role cannot be updated from {}",
                role_to_be_updated.get_role_id()
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
            .store
            .update_user_role_by_user_id_and_lineage(
                user_to_be_updated.get_user_id(),
                &user_from_token.org_id,
                &user_from_token.merchant_id,
                None,
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
        .store
        .find_user_role_by_user_id_and_lineage(
            user_to_be_updated.get_user_id(),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            None,
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
        let role_to_be_updated = roles::RoleInfo::from_role_id(
            &state,
            &user_role.role_id,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

        if !role_to_be_updated.is_updatable() {
            return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
                "User role cannot be updated from {}",
                role_to_be_updated.get_role_id()
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
            .store
            .update_user_role_by_user_id_and_lineage(
                user_to_be_updated.get_user_id(),
                &user_from_token.org_id,
                &user_from_token.merchant_id,
                None,
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

pub async fn accept_invitation(
    state: SessionState,
    user_token: auth::UserFromToken,
    req: user_role_api::AcceptInvitationRequest,
) -> UserResponse<()> {
    let merchant_accounts = state
        .store
        .list_multiple_merchant_accounts(&(&state).into(), req.merchant_ids)
        .await
        .change_context(UserErrors::InternalServerError)?;
    let update_result =
        futures::future::join_all(merchant_accounts.iter().map(|merchant_account| async {
            let (update_v1_result, update_v2_result) =
                utils::user_role::update_v1_and_v2_user_roles_in_db(
                    &state,
                    user_token.user_id.as_str(),
                    &merchant_account.organization_id,
                    merchant_account.get_id(),
                    None,
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
        }))
        .await;

    if update_result.is_empty() || update_result.iter().all(Result::is_err) {
        return Err(UserErrors::MerchantIdNotFound.into());
    }

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
                    org_id,
                    merchant_id,
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

pub async fn merchant_select_token_only_flow(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_role_api::MerchantSelectRequest,
) -> UserResponse<user_api::TokenResponse> {
    let merchant_accounts = state
        .store
        .list_multiple_merchant_accounts(&(&state).into(), req.merchant_ids)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let update_result =
        futures::future::join_all(merchant_accounts.iter().map(|merchant_account| async {
            let (update_v1_result, update_v2_result) =
                utils::user_role::update_v1_and_v2_user_roles_in_db(
                    &state,
                    user_token.user_id.as_str(),
                    &merchant_account.organization_id,
                    merchant_account.get_id(),
                    None,
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
        }))
        .await;

    if update_result.is_empty() || update_result.iter().all(Result::is_err) {
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

pub async fn accept_invitations_pre_auth(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_role_api::AcceptInvitationsPreAuthRequest,
) -> UserResponse<user_api::TokenResponse> {
    let lineages = futures::future::try_join_all(req.into_iter().map(|entity| {
        utils::user_role::get_lineage_for_user_id_and_entity_for_accepting_invite(
            &state,
            &user_token.user_id,
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
                    org_id,
                    merchant_id,
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
        .find_user_by_email(&domain::UserEmail::from_pii_email(request.email)?.into_inner())
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

    let deletion_requestor_role_info = roles::RoleInfo::from_role_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let mut user_role_deleted_flag = false;

    // Find in V2
    let user_role_v2 = match state
        .store
        .find_user_role_by_user_id_and_lineage(
            user_from_db.get_user_id(),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            user_from_token.profile_id.as_ref(),
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
        let target_role_info = roles::RoleInfo::from_role_id(
            &state,
            &role_to_be_deleted.role_id,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
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
            .store
            .delete_user_role_by_user_id_and_lineage(
                user_from_db.get_user_id(),
                &user_from_token.org_id,
                &user_from_token.merchant_id,
                user_from_token.profile_id.as_ref(),
                UserRoleVersion::V2,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user role")?;
    }

    // Find in V1
    let user_role_v1 = match state
        .store
        .find_user_role_by_user_id_and_lineage(
            user_from_db.get_user_id(),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            user_from_token.profile_id.as_ref(),
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
        let target_role_info = roles::RoleInfo::from_role_id(
            &state,
            &role_to_be_deleted.role_id,
            &user_from_token.merchant_id,
            &user_from_token.org_id,
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
            .store
            .delete_user_role_by_user_id_and_lineage(
                user_from_db.get_user_id(),
                &user_from_token.org_id,
                &user_from_token.merchant_id,
                user_from_token.profile_id.as_ref(),
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
    let user_roles_v2 = state
        .store
        .list_user_roles_by_user_id_and_version(user_from_db.get_user_id(), UserRoleVersion::V2)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let user_roles_v1 = state
        .store
        .list_user_roles_by_user_id_and_version(user_from_db.get_user_id(), UserRoleVersion::V1)
        .await
        .change_context(UserErrors::InternalServerError)?;

    // If user has no more role associated with him then deleting user
    if user_roles_v2.is_empty() && user_roles_v1.is_empty() {
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
) -> UserResponse<Vec<user_role_api::ListUsersInEntityResponse>> {
    let requestor_role_info = roles::RoleInfo::from_role_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let user_roles_set: HashSet<_> = match requestor_role_info.get_entity_type() {
        EntityType::Organization => state
            .store
            .list_user_roles_by_org_id(ListUserRolesByOrgIdPayload {
                user_id: None,
                org_id: &user_from_token.org_id,
                merchant_id: None,
                profile_id: None,
                version: None,
            })
            .await
            .change_context(UserErrors::InternalServerError)?
            .into_iter()
            .collect(),
        EntityType::Merchant => state
            .store
            .list_user_roles_by_org_id(ListUserRolesByOrgIdPayload {
                user_id: None,
                org_id: &user_from_token.org_id,
                merchant_id: Some(&user_from_token.merchant_id),
                profile_id: None,
                version: None,
            })
            .await
            .change_context(UserErrors::InternalServerError)?
            .into_iter()
            .collect(),
        EntityType::Profile => {
            let Some(profile_id) = user_from_token.profile_id.as_ref() else {
                return Err(UserErrors::JwtProfileIdMissing.into());
            };

            state
                .store
                .list_user_roles_by_org_id(ListUserRolesByOrgIdPayload {
                    user_id: None,
                    org_id: &user_from_token.org_id,
                    merchant_id: Some(&user_from_token.merchant_id),
                    profile_id: Some(profile_id),
                    version: None,
                })
                .await
                .change_context(UserErrors::InternalServerError)?
                .into_iter()
                .collect()
        }
        EntityType::Internal => HashSet::new(),
    };

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
                        .map(|role_id| user_role_api::role::MinimalRoleInfo {
                            role_id,
                            role_name: None,
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

pub async fn list_invitations_for_user(
    state: SessionState,
    user_from_token: auth::UserIdFromAuth,
) -> UserResponse<Vec<user_role_api::ListInvitationForUserResponse>> {
    let invitations = state
        .store
        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
            user_id: &user_from_token.user_id,
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
        .collect::<HashSet<_>>()
        .into_iter()
        .filter_map(|user_role| {
            let (entity_id, entity_type) = user_role.get_entity_id_and_type()?;
            Some(user_role_api::ListInvitationForUserResponse {
                entity_id,
                entity_type,
                entity_name: None,
                role_id: user_role.role_id,
            })
        })
        .collect();

    Ok(ApplicationResponse::Json(invitations))
}
