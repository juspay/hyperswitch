use api_models::{user as user_api, user_role as user_role_api};
use diesel_models::{enums::UserStatus, user_role::UserRoleUpdate};
use error_stack::ResultExt;
use masking::ExposeInterface;
use router_env::logger;

use crate::{
    consts,
    core::errors::{StorageErrorExt, UserErrors, UserResponse},
    routes::AppState,
    services::{
        authentication::{self as auth},
        authorization::{info, predefined_permissions},
        ApplicationResponse,
    },
    types::domain,
    utils,
};

pub async fn get_authorization_info(
    _state: AppState,
) -> UserResponse<user_role_api::AuthorizationInfoResponse> {
    Ok(ApplicationResponse::Json(
        user_role_api::AuthorizationInfoResponse(
            info::get_authorization_info()
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
    ))
}

pub async fn list_roles(_state: AppState) -> UserResponse<user_role_api::ListRolesResponse> {
    Ok(ApplicationResponse::Json(user_role_api::ListRolesResponse(
        predefined_permissions::PREDEFINED_PERMISSIONS
            .iter()
            .filter(|(_, role_info)| role_info.is_invitable())
            .filter_map(|(role_id, role_info)| {
                utils::user_role::get_role_name_and_permission_response(role_info).map(
                    |(permissions, role_name)| user_role_api::RoleInfoResponse {
                        permissions,
                        role_id,
                        role_name,
                    },
                )
            })
            .collect(),
    )))
}

pub async fn get_role(
    _state: AppState,
    role: user_role_api::GetRoleRequest,
) -> UserResponse<user_role_api::RoleInfoResponse> {
    let info = predefined_permissions::PREDEFINED_PERMISSIONS
        .get_key_value(role.role_id.as_str())
        .and_then(|(role_id, role_info)| {
            utils::user_role::get_role_name_and_permission_response(role_info).map(
                |(permissions, role_name)| user_role_api::RoleInfoResponse {
                    permissions,
                    role_id,
                    role_name,
                },
            )
        })
        .ok_or(UserErrors::InvalidRoleId)?;

    Ok(ApplicationResponse::Json(info))
}

pub async fn get_role_from_token(
    _state: AppState,
    user: auth::UserFromToken,
) -> UserResponse<Vec<user_role_api::Permission>> {
    Ok(ApplicationResponse::Json(
        predefined_permissions::PREDEFINED_PERMISSIONS
            .get(user.role_id.as_str())
            .ok_or(UserErrors::InternalServerError.into())
            .attach_printable("Invalid Role Id in JWT")?
            .get_permissions()
            .iter()
            .map(|&per| per.into())
            .collect(),
    ))
}

pub async fn update_user_role(
    state: AppState,
    user_from_token: auth::UserFromToken,
    req: user_role_api::UpdateUserRoleRequest,
) -> UserResponse<()> {
    if !predefined_permissions::is_role_updatable(&req.role_id)? {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable(format!("User role cannot be updated to {}", req.role_id));
    }

    let user_to_be_updated =
        utils::user::get_user_from_db_by_email(&state, domain::UserEmail::try_from(req.email)?)
            .await
            .to_not_found_response(UserErrors::InvalidRoleOperation)
            .attach_printable("User not found in our records".to_string())?;

    if user_from_token.user_id == user_to_be_updated.get_user_id() {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("User Changing their own role");
    }

    let user_role_to_be_updated = user_to_be_updated
        .get_role_from_db_by_merchant_id(&state, &user_from_token.merchant_id)
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)?;

    if !predefined_permissions::is_role_updatable(&user_role_to_be_updated.role_id)? {
        return Err(UserErrors::InvalidRoleOperation.into()).attach_printable(format!(
            "User role cannot be updated from {}",
            user_role_to_be_updated.role_id
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
) -> UserResponse<user_api::DashboardEntryResponse> {
    if user_from_token.role_id != consts::user_role::ROLE_ID_ORGANIZATION_ADMIN {
        return Err(UserErrors::InvalidRoleOperation.into()).attach_printable(format!(
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
        return Err(UserErrors::InvalidRoleOperation.into())
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

    let user_from_db = domain::UserFromStorage::from(user_from_token.get_user_from_db(&state).await?);
    let user_role = user_from_db
        .get_role_from_db_by_merchant_id(&state, &user_from_token.merchant_id)
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)?;

    let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;

    Ok(ApplicationResponse::Json(
        utils::user::get_dashboard_entry_response(&state, user_from_db, user_role, token)?,
    ))
}

pub async fn accept_invitation(
    state: AppState,
    user_token: auth::UserWithoutMerchantFromToken,
    req: user_role_api::AcceptInvitationRequest,
) -> UserResponse<user_role_api::AcceptInvitationResponse> {
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

        let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;
        return Ok(ApplicationResponse::Json(
            utils::user::get_dashboard_entry_response(&state, user_from_db, user_role, token)?,
        ));
    }

    Ok(ApplicationResponse::StatusOk)
}

pub async fn delete_user_role(
    state: AppState,
    user_from_token: auth::UserFromToken,
    request: user_role_api::DeleteUserRoleRequest,
) -> UserResponse<()> {
    let user_from_db: domain::UserFromStorage = state
        .store
        .find_user_by_email(
            domain::UserEmail::from_pii_email(request.email)?
                .get_secret()
                .expose()
                .as_str(),
        )
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
        return Err(UserErrors::InvalidDeleteOperation.into())
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
            if !predefined_permissions::is_role_deletable(&user_role.role_id)? {
                return Err(UserErrors::InvalidDeleteOperation.into())
                    .attach_printable(format!("role_id = {} is not deletable", user_role.role_id));
            }
        }
        None => {
            return Err(UserErrors::InvalidDeleteOperation.into())
                .attach_printable("User is not associated with the merchant");
        }
    };

    if user_roles.len() > 1 {
        state
            .store
            .delete_user_role_by_user_id_merchant_id(
                user_from_db.get_user_id(),
                user_from_token.merchant_id.as_str(),
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Error while deleting user role")?;

        Ok(ApplicationResponse::StatusOk)
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
            .attach_printable("Error while deleting user role")?;

        Ok(ApplicationResponse::StatusOk)
    }
}
