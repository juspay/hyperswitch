use api_models::{user as user_api, user_role as user_role_api};
use diesel_models::{enums::UserStatus, user as storage, user_role::UserRoleUpdate};
use error_stack::ResultExt;

use crate::{
    consts,
    core::errors::{UserErrors, UserResponse},
    routes::AppState,
    services::{
        authentication::{self as auth},
        authorization::{info, predefined_permissions},
        ApplicationResponse,
    },
    types::domain,
    utils,
};

pub async fn create_internal_user(
    state: AppState,
    request: user_role_api::CreateInternalUserRequest,
) -> UserResponse<()> {
    let new_user = domain::NewUser::try_from(request)?;

    let mut store_user: storage::UserNew = new_user.clone().try_into()?;
    store_user.set_is_verified(true);

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            consts::user_role::INTERNAL_USER_MERCHANT_ID,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::MerchantIdNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    state
        .store
        .find_merchant_account_by_merchant_id(
            consts::user_role::INTERNAL_USER_MERCHANT_ID,
            &key_store,
        )
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::MerchantIdNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    state
        .store
        .insert_user(store_user)
        .await
        .map_err(|e| {
            if e.current_context().is_db_unique_violation() {
                e.change_context(UserErrors::UserExists)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })
        .map(domain::user::UserFromStorage::from)?;

    new_user
        .insert_user_role_in_db(
            state,
            consts::user_role::ROLE_ID_INTERNAL_VIEW_ONLY_USER.to_string(),
            UserStatus::Active,
        )
        .await?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn switch_merchant_id(
    state: AppState,
    request: user_role_api::SwitchMerchantIdRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::ConnectAccountResponse> {
    if !utils::user_role::is_internal_role(&user_from_token.role_id) {
        let merchant_list =
            utils::user_role::get_merchant_ids_for_user(state.clone(), &user_from_token.user_id)
                .await?;
        if !merchant_list.contains(&request.merchant_id) {
            return Err(UserErrors::InvalidRoleOperation.into())
                .attach_printable("User doesn't have access to switch");
        }
    }

    if user_from_token.merchant_id == request.merchant_id {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("User switch to same merchant id.");
    }

    let user = state
        .store
        .find_user_by_id(&user_from_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            request.merchant_id.as_str(),
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::MerchantIdNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    let org_id = state
        .store
        .find_merchant_account_by_merchant_id(request.merchant_id.as_str(), &key_store)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::MerchantIdNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?
        .organization_id;

    let user = domain::UserFromStorage::from(user);
    let user_role = state
        .store
        .find_user_role_by_user_id(user.get_user_id())
        .await
        .change_context(UserErrors::InternalServerError)?;

    let token = Box::pin(user.get_jwt_auth_token_with_custom_merchant_id(
        state.clone(),
        request.merchant_id.clone(),
        org_id,
    ))
    .await?
    .into();

    Ok(ApplicationResponse::Json(
        user_api::ConnectAccountResponse {
            merchant_id: request.merchant_id,
            token,
            name: user.get_name(),
            email: user.get_email(),
            user_id: user.get_user_id().to_string(),
            verification_days_left: None,
            user_role: user_role.role_id,
        },
    ))
}

pub async fn get_authorization_info(
    _state: AppState,
) -> UserResponse<user_role_api::AuthorizationInfoResponse> {
    Ok(ApplicationResponse::Json(
        user_role_api::AuthorizationInfoResponse(
            info::get_authorization_info()
                .into_iter()
                .filter_map(|module| module.try_into().ok())
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

pub async fn create_merchant_account(
    state: AppState,
    user_from_token: auth::UserFromToken,
    req: user_role_api::UserMerchantCreate,
) -> UserResponse<()> {
    let user_from_db: domain::UserFromStorage =
        user_from_token.get_user(state.clone()).await?.into();

    let new_user = domain::NewUser::try_from((user_from_db, req, user_from_token))?;
    let new_merchant = new_user.get_new_merchant();
    new_merchant
        .create_new_merchant_and_insert_in_db(state.to_owned())
        .await?;

    let role_insertion_res = new_user
        .insert_user_role_in_db(
            state.clone(),
            consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            UserStatus::Active,
        )
        .await;
    if let Err(e) = role_insertion_res {
        let _ = state
            .store
            .delete_merchant_account_by_merchant_id(new_merchant.get_merchant_id().as_str())
            .await;
        return Err(e);
    }

    Ok(ApplicationResponse::StatusOk)
}
