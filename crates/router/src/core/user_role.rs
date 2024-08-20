use api_models::{user as user_api, user_role as user_role_api};
use diesel_models::{
    enums::{UserRoleVersion, UserStatus},
    user_role::UserRoleUpdate,
};
use error_stack::{report, ResultExt};

use crate::{
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
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

// TODO: To be deprecated once groups are stable
pub async fn get_authorization_info_with_modules(
    _state: SessionState,
) -> UserResponse<user_role_api::AuthorizationInfoResponse> {
    Ok(ApplicationResponse::Json(
        user_role_api::AuthorizationInfoResponse(
            info::get_module_authorization_info()
                .into_iter()
                .map(|module_info| user_role_api::AuthorizationInfo::Module(module_info.into()))
                .collect(),
        ),
    ))
}

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

    let user_role_to_be_updated = user_to_be_updated
        .get_role_from_db_by_merchant_id(&state, &user_from_token.merchant_id)
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)?;

    let role_to_be_updated = roles::RoleInfo::from_role_id(
        &state,
        &user_role_to_be_updated.role_id,
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

    let (updated_v1_role, updated_v2_role) = utils::user_role::update_v1_and_v2_user_roles_in_db(
        &state,
        user_to_be_updated.get_user_id(),
        &user_from_token.org_id,
        &user_from_token.merchant_id,
        UserRoleUpdate::UpdateRole {
            role_id: req.role_id.clone(),
            modified_by: user_from_token.user_id,
        },
    )
    .await;

    if updated_v1_role
        .as_ref()
        .is_err_and(|err| !err.current_context().is_db_not_found())
        || updated_v2_role
            .as_ref()
            .is_err_and(|err| !err.current_context().is_db_not_found())
    {
        return Err(report!(UserErrors::InternalServerError));
    }

    if updated_v1_role.is_err() && updated_v2_role.is_err() {
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
            let (updated_v1_role, updated_v2_role) =
                utils::user_role::update_v1_and_v2_user_roles_in_db(
                    &state,
                    user_token.user_id.as_str(),
                    &merchant_account.organization_id,
                    merchant_account.get_id(),
                    UserRoleUpdate::UpdateStatus {
                        status: UserStatus::Active,
                        modified_by: user_token.user_id.clone(),
                    },
                )
                .await;

            if updated_v1_role.is_err_and(|err| !err.current_context().is_db_not_found())
                || updated_v2_role.is_err_and(|err| !err.current_context().is_db_not_found())
            {
                Err(report!(UserErrors::InternalServerError))
            } else {
                Ok(())
            }
        }))
        .await;

    if update_result.iter().all(Result::is_err) {
        return Err(UserErrors::MerchantIdNotFound.into());
    }

    Ok(ApplicationResponse::StatusOk)
}

pub async fn merchant_select(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_role_api::MerchantSelectRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::DashboardEntryResponse>> {
    let merchant_accounts = state
        .store
        .list_multiple_merchant_accounts(&(&state).into(), req.merchant_ids)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let update_result =
        futures::future::join_all(merchant_accounts.iter().map(|merchant_account| async {
            let (updated_v1_role, updated_v2_role) =
                utils::user_role::update_v1_and_v2_user_roles_in_db(
                    &state,
                    user_token.user_id.as_str(),
                    &merchant_account.organization_id,
                    merchant_account.get_id(),
                    UserRoleUpdate::UpdateStatus {
                        status: UserStatus::Active,
                        modified_by: user_token.user_id.clone(),
                    },
                )
                .await;

            if updated_v1_role.is_err_and(|err| !err.current_context().is_db_not_found())
                || updated_v2_role.is_err_and(|err| !err.current_context().is_db_not_found())
            {
                Err(report!(UserErrors::InternalServerError))
            } else {
                Ok(())
            }
        }))
        .await;

    if update_result.iter().all(Result::is_err) {
        return Err(UserErrors::MerchantIdNotFound.into());
    }

    if let Some(true) = req.need_dashboard_entry_response {
        let user_from_db: domain::UserFromStorage = state
            .global_store
            .find_user_by_id(user_token.user_id.as_str())
            .await
            .change_context(UserErrors::InternalServerError)?
            .into();

        let user_role = user_from_db.get_role_from_db(state.clone()).await?;

        utils::user_role::set_role_permissions_in_cache_by_user_role(&state, &user_role).await;

        let token =
            utils::user::generate_jwt_auth_token_without_profile(&state, &user_from_db, &user_role)
                .await?;
        let response = utils::user::get_dashboard_entry_response(
            &state,
            user_from_db,
            user_role,
            token.clone(),
        )?;
        return auth::cookies::set_cookie_response(
            user_api::TokenOrPayloadResponse::Payload(response),
            token,
        );
    }

    Ok(ApplicationResponse::StatusOk)
}

pub async fn merchant_select_token_only_flow(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_role_api::MerchantSelectRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::DashboardEntryResponse>> {
    let merchant_accounts = state
        .store
        .list_multiple_merchant_accounts(&(&state).into(), req.merchant_ids)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let update_result =
        futures::future::join_all(merchant_accounts.iter().map(|merchant_account| async {
            let (updated_v1_role, updated_v2_role) =
                utils::user_role::update_v1_and_v2_user_roles_in_db(
                    &state,
                    user_token.user_id.as_str(),
                    &merchant_account.organization_id,
                    merchant_account.get_id(),
                    UserRoleUpdate::UpdateStatus {
                        status: UserStatus::Active,
                        modified_by: user_token.user_id.clone(),
                    },
                )
                .await;

            if updated_v1_role.is_err_and(|err| !err.current_context().is_db_not_found())
                || updated_v2_role.is_err_and(|err| !err.current_context().is_db_not_found())
            {
                Err(report!(UserErrors::InternalServerError))
            } else {
                Ok(())
            }
        }))
        .await;

    if update_result.iter().all(Result::is_err) {
        return Err(UserErrors::MerchantIdNotFound.into());
    }

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(user_token.user_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let user_role = user_from_db.get_role_from_db(state.clone()).await?;

    let current_flow =
        domain::CurrentFlow::new(user_token, domain::SPTFlow::MerchantSelect.into())?;
    let next_flow = current_flow.next(user_from_db.clone(), &state).await?;

    let token = next_flow
        .get_token_with_user_role(&state, &user_role)
        .await?;

    let response = user_api::TokenOrPayloadResponse::Token(user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    });
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
        .list_user_roles_by_user_id(user_from_db.get_user_id(), UserRoleVersion::V2)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let user_roles_v1 = state
        .store
        .list_user_roles_by_user_id(user_from_db.get_user_id(), UserRoleVersion::V1)
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
