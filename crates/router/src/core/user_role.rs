use api_models::{user as user_api, user_role as user_role_api};
use diesel_models::{enums::UserStatus, user_role::UserRoleUpdate};
use error_stack::{report, ResultExt};
use router_env::logger;

use crate::{
    consts,
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    routes::{app::ReqState, AppState},
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
    _state: AppState,
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
    _state: AppState,
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
    state: AppState,
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

    state
        .store
        .update_user_role_by_user_id_merchant_id(
            user_to_be_updated.get_user_id(),
            user_role_to_be_updated.merchant_id.as_str(),
            UserRoleUpdate::UpdateRole {
                role_id: req.role_id.clone(),
                modified_by: user_from_token.user_id,
            },
        )
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)
        .attach_printable("User with given email is not found in the organization")?;

    auth::blacklist::insert_user_in_blacklist(&state, user_to_be_updated.get_user_id()).await?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn transfer_org_ownership(
    state: AppState,
    user_from_token: auth::UserFromToken,
    req: user_role_api::TransferOrgOwnershipRequest,
    _req_state: ReqState,
) -> UserResponse<user_api::DashboardEntryResponse> {
    if user_from_token.role_id != consts::user_role::ROLE_ID_ORGANIZATION_ADMIN {
        return Err(report!(UserErrors::InvalidRoleOperation)).attach_printable(format!(
            "role_id = {} is not org_admin",
            user_from_token.role_id
        ));
    }

    let user_to_be_updated =
        utils::user::get_user_from_db_by_email(&state, domain::UserEmail::try_from(req.email)?)
            .await
            .to_not_found_response(UserErrors::InvalidRoleOperation)
            .attach_printable("User not found in our records".to_string())?;

    if user_from_token.user_id == user_to_be_updated.get_user_id() {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("User transferring ownership to themselves".to_string());
    }

    state
        .store
        .transfer_org_ownership_between_users(
            &user_from_token.user_id,
            user_to_be_updated.get_user_id(),
            &user_from_token.org_id,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    auth::blacklist::insert_user_in_blacklist(&state, user_to_be_updated.get_user_id()).await?;
    auth::blacklist::insert_user_in_blacklist(&state, &user_from_token.user_id).await?;

    let user_from_db = user_from_token.get_user_from_db(&state).await?;
    let user_role = user_from_db
        .get_role_from_db_by_merchant_id(&state, &user_from_token.merchant_id)
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)?;

    utils::user_role::set_role_permissions_in_cache_by_user_role(&state, &user_role).await;

    let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;
    let response =
        utils::user::get_dashboard_entry_response(&state, user_from_db, user_role, token.clone())?;

    auth::cookies::set_cookie_response(response, token)
}

pub async fn accept_invitation(
    state: AppState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_role_api::AcceptInvitationRequest,
    _req_state: ReqState,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::DashboardEntryResponse>> {
    let user_role = futures::future::join_all(req.merchant_ids.iter().map(|merchant_id| async {
        state
            .store
            .update_user_role_by_user_id_merchant_id(
                user_token.user_id.as_str(),
                merchant_id,
                UserRoleUpdate::UpdateStatus {
                    status: UserStatus::Active,
                    modified_by: user_token.user_id.clone(),
                },
            )
            .await
            .map_err(|e| {
                logger::error!("Error while accepting invitation {}", e);
            })
            .ok()
    }))
    .await
    .into_iter()
    .reduce(Option::or)
    .flatten()
    .ok_or(UserErrors::MerchantIdNotFound)?;

    if let Some(true) = req.need_dashboard_entry_response {
        let user_from_db = state
            .store
            .find_user_by_id(user_token.user_id.as_str())
            .await
            .change_context(UserErrors::InternalServerError)?
            .into();

        utils::user_role::set_role_permissions_in_cache_by_user_role(&state, &user_role).await;

        let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;

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

pub async fn accept_invitation_token_only_flow(
    state: AppState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_role_api::AcceptInvitationRequest,
    _req_state: ReqState,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::DashboardEntryResponse>> {
    let user_role = futures::future::join_all(req.merchant_ids.iter().map(|merchant_id| async {
        state
            .store
            .update_user_role_by_user_id_merchant_id(
                user_token.user_id.as_str(),
                merchant_id,
                UserRoleUpdate::UpdateStatus {
                    status: UserStatus::Active,
                    modified_by: user_token.user_id.clone(),
                },
            )
            .await
            .map_err(|e| {
                logger::error!("Error while accepting invitation {}", e);
            })
            .ok()
    }))
    .await
    .into_iter()
    .reduce(Option::or)
    .flatten()
    .ok_or(UserErrors::MerchantIdNotFound)?;

    let user_from_db: domain::UserFromStorage = state
        .store
        .find_user_by_id(user_token.user_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let current_flow = domain::CurrentFlow::new(
        user_token.origin,
        domain::UserFlow::SPTFlow(domain::SPTFlow::AcceptInvitationFromEmail),
    )?;
    let next_flow = current_flow.next(user_from_db.clone(), &state).await?;

    let token = match next_flow.get_flow() {
        domain::UserFlow::SPTFlow(spt_flow) => spt_flow.generate_spt(&state, &next_flow).await,
        domain::UserFlow::JWTFlow(jwt_flow) => {
            #[cfg(feature = "email")]
            {
                user_from_db.get_verification_days_left(&state)?;
            }
            jwt_flow.generate_jwt(&state, &next_flow, &user_role).await
        }
    }?;

    let response = user_api::TokenOrPayloadResponse::Token(user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    });
    auth::cookies::set_cookie_response(response, token)
}

pub async fn delete_user_role(
    state: AppState,
    user_from_token: auth::UserFromToken,
    request: user_role_api::DeleteUserRoleRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let user_from_db: domain::UserFromStorage = state
        .store
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

    let user_roles = state
        .store
        .list_user_roles_by_user_id(user_from_db.get_user_id())
        .await
        .change_context(UserErrors::InternalServerError)?;

    match user_roles
        .iter()
        .find(|&role| role.merchant_id == user_from_token.merchant_id.as_str())
    {
        Some(user_role) => {
            let role_info = roles::RoleInfo::from_role_id(
                &state,
                &user_role.role_id,
                &user_from_token.merchant_id,
                &user_from_token.org_id,
            )
            .await
            .change_context(UserErrors::InternalServerError)?;
            if !role_info.is_deletable() {
                return Err(report!(UserErrors::InvalidDeleteOperation))
                    .attach_printable(format!("role_id = {} is not deletable", user_role.role_id));
            }
        }
        None => {
            return Err(report!(UserErrors::InvalidDeleteOperation))
                .attach_printable("User is not associated with the merchant");
        }
    };

    let deleted_user_role = if user_roles.len() > 1 {
        state
            .store
            .delete_user_role_by_user_id_merchant_id(
                user_from_db.get_user_id(),
                user_from_token.merchant_id.as_str(),
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user role")?
    } else {
        state
            .store
            .delete_user_by_user_id(user_from_db.get_user_id())
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user entry")?;

        state
            .store
            .delete_user_role_by_user_id_merchant_id(
                user_from_db.get_user_id(),
                user_from_token.merchant_id.as_str(),
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user role")?
    };

    auth::blacklist::insert_user_in_blacklist(&state, &deleted_user_role.user_id).await?;
    Ok(ApplicationResponse::StatusOk)
}
