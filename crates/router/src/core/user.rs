use api_models::user as user_api;
use diesel_models::{enums::UserStatus, user as storage_user};
use error_stack::{IntoReport, ResultExt};
use masking::{ExposeInterface, Secret};
use router_env::env;

use super::errors::{UserErrors, UserResponse};
use crate::{
    consts,
    db::user::UserInterface,
    routes::AppState,
    services::{authentication as auth, ApplicationResponse},
    types::domain,
    utils,
};

pub mod dashboard_metadata;

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

        user_from_db.compare_password(request.password)?;

        let user_role = user_from_db.get_role_from_db(state.clone()).await?;
        let jwt_token = user_from_db
            .get_jwt_auth_token(state.clone(), user_role.org_id)
            .await?;

        return Ok(ApplicationResponse::Json(
            user_api::ConnectAccountResponse {
                token: Secret::new(jwt_token),
                merchant_id: user_role.merchant_id,
                name: user_from_db.get_name(),
                email: user_from_db.get_email(),
                verification_days_left: None,
                user_role: user_role.role_id,
                user_id: user_from_db.get_user_id().to_string(),
            },
        ));
    } else if find_user
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
        let jwt_token = user_from_db
            .get_jwt_auth_token(state.clone(), user_role.org_id)
            .await?;

        #[cfg(feature = "email")]
        {
            use router_env::logger;

            use crate::services::email::types as email_types;

            let email_contents = email_types::WelcomeEmail {
                recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
                settings: state.conf.clone(),
            };

            let send_email_result = state
                .email_client
                .compose_and_send_email(
                    Box::new(email_contents),
                    state.conf.proxy.https_url.as_ref(),
                )
                .await;

            logger::info!(?send_email_result);
        }

        return Ok(ApplicationResponse::Json(
            user_api::ConnectAccountResponse {
                token: Secret::new(jwt_token),
                merchant_id: user_role.merchant_id,
                name: user_from_db.get_name(),
                email: user_from_db.get_email(),
                verification_days_left: None,
                user_role: user_role.role_id,
                user_id: user_from_db.get_user_id().to_string(),
            },
        ));
    } else {
        Err(UserErrors::InternalServerError.into())
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
