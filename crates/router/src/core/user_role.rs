use api_models::user_role as user_role_api;
use diesel_models::{enums::UserStatus, user_role::UserRoleUpdate};
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    core::errors::{UserErrors, UserResponse},
    routes::AppState,
    services::{
        authentication::{self as auth},
        authorization::{info, predefined_permissions},
        ApplicationResponse,
    },
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
    let merchant_id = user_from_token.merchant_id;
    let role_id = req.role_id.clone();
    utils::user_role::validate_role_id(role_id.as_str())?;

    if user_from_token.user_id == req.user_id {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("Admin User Changing their role");
    }

    state
        .store
        .update_user_role_by_user_id_merchant_id(
            req.user_id.as_str(),
            merchant_id.as_str(),
            UserRoleUpdate::UpdateRole {
                role_id,
                modified_by: user_from_token.user_id,
            },
        )
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                return e
                    .change_context(UserErrors::InvalidRoleOperation)
                    .attach_printable("UserId MerchantId not found");
            }
            e.change_context(UserErrors::InternalServerError)
        })?;

    Ok(ApplicationResponse::StatusOk)
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
