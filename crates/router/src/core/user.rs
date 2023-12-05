use api_models::user as user_api;
use diesel_models::{enums::UserStatus, user as storage_user};
#[cfg(feature = "email")]
use error_stack::IntoReport;
use error_stack::ResultExt;
use masking::ExposeInterface;
#[cfg(feature = "email")]
use router_env::env;
#[cfg(feature = "email")]
use router_env::logger;

use super::errors::{UserErrors, UserResponse};
#[cfg(feature = "email")]
use crate::services::email::types as email_types;
use crate::{
    consts,
    db::user::UserInterface,
    routes::AppState,
    services::{authentication as auth, ApplicationResponse},
    types::domain,
    utils,
};
pub mod dashboard_metadata;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;

#[cfg(feature = "email")]
pub async fn signup_with_merchant_id(
    state: AppState,
    request: user_api::SignUpWithMerchantIdRequest,
) -> UserResponse<user_api::SignUpWithMerchantIdResponse> {
    let new_user = domain::NewUser::try_from(request.clone())?;
    new_user
        .get_new_merchant()
        .get_new_organization()
        .insert_org_in_db(state.clone())
        .await?;

    let user_from_db = new_user
        .insert_user_and_merchant_in_db(state.clone())
        .await?;

    let user_role = new_user
        .insert_user_role_in_db(
            state.clone(),
            consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            UserStatus::Active,
        )
        .await?;

    let email_contents = email_types::ResetPassword {
        recipient_email: user_from_db.get_email().try_into()?,
        user_name: domain::UserName::new(user_from_db.get_name())?,
        settings: state.conf.clone(),
        subject: "Get back to Hyperswitch - Reset Your Password Now",
    };

    let send_email_result = state
        .email_client
        .compose_and_send_email(
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await;

    logger::info!(?send_email_result);
    Ok(ApplicationResponse::Json(user_api::AuthorizeResponse {
        is_email_sent: send_email_result.is_ok(),
        user_id: user_from_db.get_user_id().to_string(),
        merchant_id: user_role.merchant_id,
    }))
}

pub async fn signup(
    state: AppState,
    request: user_api::SignUpRequest,
) -> UserResponse<user_api::SignUpResponse> {
    let new_user = domain::NewUser::try_from(request)?;
    new_user
        .get_new_merchant()
        .get_new_organization()
        .insert_org_in_db(state.clone())
        .await?;
    let user_from_db = new_user
        .insert_user_and_merchant_in_db(state.clone())
        .await?;
    let user_role = new_user
        .insert_user_role_in_db(
            state.clone(),
            consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            UserStatus::Active,
        )
        .await?;
    let token = utils::user::generate_jwt_auth_token(state, &user_from_db, &user_role).await?;

    Ok(ApplicationResponse::Json(
        utils::user::get_dashboard_entry_response(user_from_db, user_role, token),
    ))
}

pub async fn signin(
    state: AppState,
    request: user_api::SignInRequest,
) -> UserResponse<user_api::SignInResponse> {
    let user_from_db: domain::UserFromStorage = state
        .store
        .find_user_by_email(request.email.clone().expose().expose().as_str())
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::InvalidCredentials)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?
        .into();

    user_from_db.compare_password(request.password)?;

    let user_role = user_from_db.get_role_from_db(state.clone()).await?;
    let token = utils::user::generate_jwt_auth_token(state, &user_from_db, &user_role).await?;

    Ok(ApplicationResponse::Json(
        utils::user::get_dashboard_entry_response(user_from_db, user_role, token),
    ))
}

#[cfg(feature = "email")]
pub async fn connect_account(
    state: AppState,
    request: user_api::ConnectAccountRequest,
) -> UserResponse<user_api::ConnectAccountResponse> {
    let find_user = state
        .store
        .find_user_by_email(request.email.clone().expose().expose().as_str())
        .await;

    if let Ok(found_user) = find_user {
        let user_from_db: domain::UserFromStorage = found_user.into();
        let user_role = user_from_db.get_role_from_db(state.clone()).await?;

        let email_contents = email_types::MagicLink {
            recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
            settings: state.conf.clone(),
            user_name: domain::UserName::new(user_from_db.get_name())?,
            subject: "Unlock Hyperswitch: Use Your Magic Link to Sign In",
        };

        let send_email_result = state
            .email_client
            .compose_and_send_email(
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
            .await;

        logger::info!(?send_email_result);

        return Ok(ApplicationResponse::Json(
            user_api::ConnectAccountResponse {
                is_email_sent: send_email_result.is_ok(),
                user_id: user_from_db.get_user_id().to_string(),
                merchant_id: user_role.merchant_id,
            },
        ));
    } else if find_user
        .as_ref()
        .map_err(|e| e.current_context().is_db_not_found())
        .err()
        .unwrap_or(false)
    {
        if matches!(env::which(), env::Env::Production) {
            return Err(UserErrors::InvalidCredentials).into_report();
        }

        let new_user = domain::NewUser::try_from(request)?;
        let _ = new_user
            .get_new_merchant()
            .get_new_organization()
            .insert_org_in_db(state.clone())
            .await?;
        let user_from_db = new_user
            .insert_user_and_merchant_in_db(state.clone())
            .await?;
        let user_role = new_user
            .insert_user_role_in_db(
                state.clone(),
                consts::user_role::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                UserStatus::Active,
            )
            .await?;

        let email_contents = email_types::VerifyEmail {
            recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
            settings: state.conf.clone(),
            subject: "Welcome to the Hyperswitch community!",
        };

        let send_email_result = state
            .email_client
            .compose_and_send_email(
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
            .await;

        logger::info!(?send_email_result);

        return Ok(ApplicationResponse::Json(
            user_api::ConnectAccountResponse {
                is_email_sent: send_email_result.is_ok(),
                user_id: user_from_db.get_user_id().to_string(),
                merchant_id: user_role.merchant_id,
            },
        ));
    } else {
        Err(find_user
            .err()
            .map(|e| e.change_context(UserErrors::InternalServerError))
            .unwrap_or(UserErrors::InternalServerError.into()))
    }
}

pub async fn change_password(
    state: AppState,
    request: user_api::ChangePasswordRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<()> {
    let user: domain::UserFromStorage =
        UserInterface::find_user_by_id(&*state.store, &user_from_token.user_id)
            .await
            .change_context(UserErrors::InternalServerError)?
            .into();

    user.compare_password(request.old_password)
        .change_context(UserErrors::InvalidOldPassword)?;

    let new_password_hash =
        crate::utils::user::password::generate_password_hash(request.new_password)?;

    let _ = UserInterface::update_user_by_user_id(
        &*state.store,
        user.get_user_id(),
        diesel_models::user::UserUpdate::AccountUpdate {
            name: None,
            password: Some(new_password_hash),
            is_verified: None,
        },
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn create_internal_user(
    state: AppState,
    request: user_api::CreateInternalUserRequest,
) -> UserResponse<()> {
    let new_user = domain::NewUser::try_from(request)?;

    let mut store_user: storage_user::UserNew = new_user.clone().try_into()?;
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
    request: user_api::SwitchMerchantIdRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::SwitchMerchantResponse> {
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

    let _org_id = state
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

    let token = utils::user::generate_jwt_auth_token_with_custom_merchant_id(
        state,
        &user,
        &user_role,
        request.merchant_id.clone(),
    )
    .await?;

    Ok(ApplicationResponse::Json(
        user_api::SwitchMerchantResponse {
            token,
            name: user.get_name(),
            email: user.get_email(),
            user_id: user.get_user_id().to_string(),
            verification_days_left: None,
            user_role: user_role.role_id,
            merchant_id: user_role.merchant_id,
        },
    ))
}

pub async fn create_merchant_account(
    state: AppState,
    user_from_token: auth::UserFromToken,
    req: user_api::UserMerchantCreate,
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

pub async fn list_merchant_ids_for_user(
    state: AppState,
    user: auth::UserFromToken,
) -> UserResponse<Vec<String>> {
    Ok(ApplicationResponse::Json(
        utils::user::get_merchant_ids_for_user(state, &user.user_id).await?,
    ))
}

pub async fn get_users_for_merchant_account(
    state: AppState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::GetUsersResponse> {
    let users = state
        .store
        .find_users_and_roles_by_merchant_id(user_from_token.merchant_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("No users for given merchant id")?
        .into_iter()
        .filter_map(|(user, role)| domain::UserAndRoleJoined(user, role).try_into().ok())
        .collect();

    Ok(ApplicationResponse::Json(user_api::GetUsersResponse(users)))
}
