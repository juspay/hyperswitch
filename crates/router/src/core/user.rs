use std::collections::HashMap;

use api_models::{
    payments::RedirectionResponse,
    user::{self as user_api, InviteMultipleUserResponse},
};
use common_utils::ext_traits::ValueExt;
#[cfg(feature = "email")]
use diesel_models::user_role::UserRoleUpdate;
use diesel_models::{
    enums::{TotpStatus, UserStatus},
    user as storage_user,
    user_authentication_method::{UserAuthenticationMethodNew, UserAuthenticationMethodUpdate},
    user_role::UserRoleNew,
};
use error_stack::{report, ResultExt};
#[cfg(feature = "email")]
use external_services::email::EmailData;
use masking::{ExposeInterface, PeekInterface, Secret};
#[cfg(feature = "email")]
use router_env::env;
use router_env::logger;
#[cfg(not(feature = "email"))]
use user_api::dashboard_metadata::SetMetaDataRequest;

use super::errors::{StorageErrorExt, UserErrors, UserResponse, UserResult};
#[cfg(feature = "email")]
use crate::services::email::types as email_types;
use crate::{
    consts,
    routes::{app::ReqState, SessionState},
    services::{authentication as auth, authorization::roles, openidconnect, ApplicationResponse},
    types::{domain, transformers::ForeignInto},
    utils::{self, user::two_factor_auth as tfa_utils},
};

pub mod dashboard_metadata;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;

#[cfg(feature = "email")]
pub async fn signup_with_merchant_id(
    state: SessionState,
    request: user_api::SignUpWithMerchantIdRequest,
    auth_id: Option<String>,
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
        auth_id,
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

pub async fn get_user_details(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::GetUserDetailsResponse> {
    let user = user_from_token.get_user_from_db(&state).await?;
    let verification_days_left = utils::user::get_verification_days_left(&state, &user)?;

    Ok(ApplicationResponse::Json(
        user_api::GetUserDetailsResponse {
            merchant_id: user_from_token.merchant_id,
            name: user.get_name(),
            email: user.get_email(),
            user_id: user.get_user_id().to_string(),
            verification_days_left,
            role_id: user_from_token.role_id,
            org_id: user_from_token.org_id,
            is_two_factor_auth_setup: user.get_totp_status() == TotpStatus::Set,
            recovery_codes_left: user.get_recovery_codes().map(|codes| codes.len()),
        },
    ))
}

pub async fn signup(
    state: SessionState,
    request: user_api::SignUpRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::SignUpResponse>> {
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
    utils::user_role::set_role_permissions_in_cache_by_user_role(&state, &user_role).await;

    let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;
    let response =
        utils::user::get_dashboard_entry_response(&state, user_from_db, user_role, token.clone())?;

    auth::cookies::set_cookie_response(user_api::TokenOrPayloadResponse::Payload(response), token)
}

pub async fn signup_token_only_flow(
    state: SessionState,
    request: user_api::SignUpRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::SignUpResponse>> {
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

    let next_flow =
        domain::NextFlow::from_origin(domain::Origin::SignUp, user_from_db.clone(), &state).await?;

    let token = next_flow
        .get_token_with_user_role(&state, &user_role)
        .await?;

    let response = user_api::TokenOrPayloadResponse::Token(user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    });
    auth::cookies::set_cookie_response(response, token)
}

pub async fn signin(
    state: SessionState,
    request: user_api::SignInRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::SignInResponse>> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(&request.email)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::InvalidCredentials)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?
        .into();

    user_from_db.compare_password(&request.password)?;

    let signin_strategy =
        if let Some(preferred_merchant_id) = user_from_db.get_preferred_merchant_id() {
            let preferred_role = user_from_db
                .get_role_from_db_by_merchant_id(&state, preferred_merchant_id.as_str())
                .await
                .to_not_found_response(UserErrors::InternalServerError)
                .attach_printable("User role with preferred_merchant_id not found")?;
            domain::SignInWithRoleStrategyType::SingleRole(domain::SignInWithSingleRoleStrategy {
                user: user_from_db,
                user_role: preferred_role,
            })
        } else {
            let user_roles = user_from_db.get_roles_from_db(&state).await?;
            domain::SignInWithRoleStrategyType::decide_signin_strategy_by_user_roles(
                user_from_db,
                user_roles,
            )
            .await?
        };

    let response = signin_strategy.get_signin_response(&state).await?;
    let token = utils::user::get_token_from_signin_response(&response);
    auth::cookies::set_cookie_response(user_api::TokenOrPayloadResponse::Payload(response), token)
}

pub async fn signin_token_only_flow(
    state: SessionState,
    request: user_api::SignInRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::SignInResponse>> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(&request.email)
        .await
        .to_not_found_response(UserErrors::InvalidCredentials)?
        .into();

    user_from_db.compare_password(&request.password)?;

    let next_flow =
        domain::NextFlow::from_origin(domain::Origin::SignIn, user_from_db.clone(), &state).await?;

    let token = next_flow.get_token(&state).await?;

    let response = user_api::TokenOrPayloadResponse::Token(user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    });
    auth::cookies::set_cookie_response(response, token)
}

#[cfg(feature = "email")]
pub async fn connect_account(
    state: SessionState,
    request: user_api::ConnectAccountRequest,
    auth_id: Option<String>,
) -> UserResponse<user_api::ConnectAccountResponse> {
    let find_user = state.global_store.find_user_by_email(&request.email).await;

    if let Ok(found_user) = find_user {
        let user_from_db: domain::UserFromStorage = found_user.into();
        let user_role = user_from_db.get_role_from_db(state.clone()).await?;

        let email_contents = email_types::MagicLink {
            recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
            settings: state.conf.clone(),
            user_name: domain::UserName::new(user_from_db.get_name())?,
            subject: "Unlock Hyperswitch: Use Your Magic Link to Sign In",
            auth_id,
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
            return Err(report!(UserErrors::InvalidCredentials));
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
            auth_id,
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

pub async fn signout(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<()> {
    tfa_utils::delete_totp_from_redis(&state, &user_from_token.user_id).await?;
    tfa_utils::delete_recovery_code_from_redis(&state, &user_from_token.user_id).await?;
    tfa_utils::delete_totp_secret_from_redis(&state, &user_from_token.user_id).await?;

    auth::blacklist::insert_user_in_blacklist(&state, &user_from_token.user_id).await?;
    auth::cookies::remove_cookie_response()
}

pub async fn change_password(
    state: SessionState,
    request: user_api::ChangePasswordRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<()> {
    let user: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_from_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    user.compare_password(&request.old_password)
        .change_context(UserErrors::InvalidOldPassword)?;

    if request.old_password == request.new_password {
        return Err(UserErrors::ChangePasswordError.into());
    }
    let new_password = domain::UserPassword::new(request.new_password)?;

    let new_password_hash =
        utils::user::password::generate_password_hash(new_password.get_secret())?;

    let _ = state
        .global_store
        .update_user_by_user_id(
            user.get_user_id(),
            diesel_models::user::UserUpdate::PasswordUpdate {
                password: new_password_hash,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    let _ = auth::blacklist::insert_user_in_blacklist(&state, user.get_user_id())
        .await
        .map_err(|e| logger::error!(?e));

    #[cfg(not(feature = "email"))]
    {
        state
            .store
            .delete_user_scoped_dashboard_metadata_by_merchant_id_data_key(
                &user_from_token.user_id,
                &user_from_token.merchant_id,
                diesel_models::enums::DashboardMetadata::IsChangePasswordRequired,
            )
            .await
            .map_err(|e| logger::error!("Error while deleting dashboard metadata {}", e))
            .ok();
    }

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "email")]
pub async fn forgot_password(
    state: SessionState,
    request: user_api::ForgotPasswordRequest,
    auth_id: Option<String>,
) -> UserResponse<()> {
    let user_email = domain::UserEmail::from_pii_email(request.email)?;

    let user_from_db = state
        .global_store
        .find_user_by_email(&user_email.into_inner())
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::UserNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })
        .map(domain::UserFromStorage::from)?;

    let email_contents = email_types::ResetPassword {
        recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
        settings: state.conf.clone(),
        user_name: domain::UserName::new(user_from_db.get_name())?,
        subject: "Get back to Hyperswitch - Reset Your Password Now",
        auth_id,
    };

    state
        .email_client
        .compose_and_send_email(
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await
        .map_err(|e| e.change_context(UserErrors::InternalServerError))?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn rotate_password(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    request: user_api::RotatePasswordRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let user: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let password = domain::UserPassword::new(request.password.to_owned())?;
    let hash_password = utils::user::password::generate_password_hash(password.get_secret())?;

    if user.compare_password(&request.password).is_ok() {
        return Err(UserErrors::ChangePasswordError.into());
    }

    let user = state
        .global_store
        .update_user_by_user_id(
            &user_token.user_id,
            storage_user::UserUpdate::PasswordUpdate {
                password: hash_password,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    let _ = auth::blacklist::insert_user_in_blacklist(&state, &user.user_id)
        .await
        .map_err(|e| logger::error!(?e));

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "email")]
pub async fn reset_password_token_only_flow(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    request: user_api::ResetPasswordRequest,
) -> UserResponse<()> {
    let token = request.token.expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(
            &email_token
                .get_email()
                .change_context(UserErrors::InternalServerError)?,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_user_id() != user_token.user_id {
        return Err(UserErrors::LinkInvalid.into());
    }

    let password = domain::UserPassword::new(request.password)?;
    let hash_password = utils::user::password::generate_password_hash(password.get_secret())?;

    let user = state
        .global_store
        .update_user_by_user_id(
            user_from_db.get_user_id(),
            storage_user::UserUpdate::PasswordUpdate {
                password: hash_password,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    if !user_from_db.is_verified() {
        let _ = state
            .global_store
            .update_user_by_user_id(
                user_from_db.get_user_id(),
                storage_user::UserUpdate::VerifyUser,
            )
            .await
            .map_err(|e| logger::error!(?e));
    }

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|e| logger::error!(?e));
    let _ = auth::blacklist::insert_user_in_blacklist(&state, &user.user_id)
        .await
        .map_err(|e| logger::error!(?e));

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "email")]
pub async fn reset_password(
    state: SessionState,
    request: user_api::ResetPasswordRequest,
) -> UserResponse<()> {
    let token = request.token.expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let password = domain::UserPassword::new(request.password)?;
    let hash_password = utils::user::password::generate_password_hash(password.get_secret())?;

    let user = state
        .global_store
        .update_user_by_email(
            &email_token
                .get_email()
                .change_context(UserErrors::InternalServerError)?,
            storage_user::UserUpdate::PasswordUpdate {
                password: hash_password,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    if let Some(inviter_merchant_id) = email_token.get_merchant_id() {
        let update_status_result = state
            .store
            .update_user_role_by_user_id_merchant_id(
                user.user_id.clone().as_str(),
                inviter_merchant_id,
                UserRoleUpdate::UpdateStatus {
                    status: UserStatus::Active,
                    modified_by: user.user_id.clone(),
                },
            )
            .await;
        logger::info!(?update_status_result);
    }

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|e| logger::error!(?e));
    let _ = auth::blacklist::insert_user_in_blacklist(&state, &user.user_id)
        .await
        .map_err(|e| logger::error!(?e));

    Ok(ApplicationResponse::StatusOk)
}

pub async fn invite_multiple_user(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    requests: Vec<user_api::InviteUserRequest>,
    req_state: ReqState,
    is_token_only: Option<bool>,
    auth_id: Option<String>,
) -> UserResponse<Vec<InviteMultipleUserResponse>> {
    if requests.len() > 10 {
        return Err(report!(UserErrors::MaxInvitationsError))
            .attach_printable("Number of invite requests must not exceed 10");
    }

    let responses = futures::future::join_all(requests.iter().map(|request| async {
        match handle_invitation(
            &state,
            &user_from_token,
            request,
            &req_state,
            is_token_only,
            &auth_id,
        )
        .await
        {
            Ok(response) => response,
            Err(error) => InviteMultipleUserResponse {
                email: request.email.clone(),
                is_email_sent: false,
                password: None,
                error: Some(error.current_context().get_error_message().to_string()),
            },
        }
    }))
    .await;

    Ok(ApplicationResponse::Json(responses))
}

async fn handle_invitation(
    state: &SessionState,
    user_from_token: &auth::UserFromToken,
    request: &user_api::InviteUserRequest,
    req_state: &ReqState,
    is_token_only: Option<bool>,
    auth_id: &Option<String>,
) -> UserResult<InviteMultipleUserResponse> {
    let inviter_user = user_from_token.get_user_from_db(state).await?;

    if inviter_user.get_email() == request.email {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User Inviting themselves".to_string(),
        )
        .into());
    }

    let role_info = roles::RoleInfo::from_role_id(
        state,
        &request.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    if !role_info.is_invitable() {
        return Err(report!(UserErrors::InvalidRoleId))
            .attach_printable(format!("role_id = {} is not invitable", request.role_id));
    }

    let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
    let invitee_user = state
        .global_store
        .find_user_by_email(&invitee_email.into_inner())
        .await;

    if let Ok(invitee_user) = invitee_user {
        handle_existing_user_invitation(
            state,
            user_from_token,
            request,
            invitee_user.into(),
            auth_id,
        )
        .await
    } else if invitee_user
        .as_ref()
        .map_err(|e| e.current_context().is_db_not_found())
        .err()
        .unwrap_or(false)
    {
        handle_new_user_invitation(
            state,
            user_from_token,
            request,
            req_state.clone(),
            is_token_only,
            auth_id,
        )
        .await
    } else {
        Err(UserErrors::InternalServerError.into())
    }
}

#[allow(unused_variables)]
async fn handle_existing_user_invitation(
    state: &SessionState,
    user_from_token: &auth::UserFromToken,
    request: &user_api::InviteUserRequest,
    invitee_user_from_db: domain::UserFromStorage,
    auth_id: &Option<String>,
) -> UserResult<InviteMultipleUserResponse> {
    let now = common_utils::date_time::now();
    state
        .store
        .insert_user_role(UserRoleNew {
            user_id: invitee_user_from_db.get_user_id().to_owned(),
            merchant_id: user_from_token.merchant_id.clone(),
            role_id: request.role_id.clone(),
            org_id: user_from_token.org_id.clone(),
            status: {
                if cfg!(feature = "email") {
                    UserStatus::InvitationSent
                } else {
                    UserStatus::Active
                }
            },
            created_by: user_from_token.user_id.clone(),
            last_modified_by: user_from_token.user_id.clone(),
            created_at: now,
            last_modified: now,
        })
        .await
        .map_err(|e| {
            if e.current_context().is_db_unique_violation() {
                e.change_context(UserErrors::UserExists)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    let is_email_sent;
    #[cfg(feature = "email")]
    {
        let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
        let email_contents = email_types::InviteRegisteredUser {
            recipient_email: invitee_email,
            user_name: domain::UserName::new(invitee_user_from_db.get_name())?,
            settings: state.conf.clone(),
            subject: "You have been invited to join Hyperswitch Community!",
            merchant_id: user_from_token.merchant_id.clone(),
            auth_id: auth_id.clone(),
        };

        is_email_sent = state
            .email_client
            .compose_and_send_email(
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
            .await
            .map(|email_result| logger::info!(?email_result))
            .map_err(|email_result| logger::error!(?email_result))
            .is_ok();
    }
    #[cfg(not(feature = "email"))]
    {
        is_email_sent = false;
    }

    Ok(InviteMultipleUserResponse {
        email: request.email.clone(),
        is_email_sent,
        password: None,
        error: None,
    })
}

#[allow(unused_variables)]
async fn handle_new_user_invitation(
    state: &SessionState,
    user_from_token: &auth::UserFromToken,
    request: &user_api::InviteUserRequest,
    req_state: ReqState,
    is_token_only: Option<bool>,
    auth_id: &Option<String>,
) -> UserResult<InviteMultipleUserResponse> {
    let new_user = domain::NewUser::try_from((request.clone(), user_from_token.clone()))?;

    new_user
        .insert_user_in_db(state.global_store.as_ref())
        .await
        .change_context(UserErrors::InternalServerError)?;

    let invitation_status = if cfg!(feature = "email") {
        UserStatus::InvitationSent
    } else {
        UserStatus::Active
    };

    let now = common_utils::date_time::now();
    state
        .store
        .insert_user_role(UserRoleNew {
            user_id: new_user.get_user_id().to_owned(),
            merchant_id: user_from_token.merchant_id.clone(),
            role_id: request.role_id.clone(),
            org_id: user_from_token.org_id.clone(),
            status: invitation_status,
            created_by: user_from_token.user_id.clone(),
            last_modified_by: user_from_token.user_id.clone(),
            created_at: now,
            last_modified: now,
        })
        .await
        .map_err(|e| {
            if e.current_context().is_db_unique_violation() {
                e.change_context(UserErrors::UserExists)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    let is_email_sent;
    // TODO: Adding this to avoid clippy lints, remove this once the token only flow is being used
    let _ = is_token_only;

    #[cfg(feature = "email")]
    {
        // TODO: Adding this to avoid clippy lints
        // Will be adding actual usage for this variable later
        let _ = req_state.clone();
        let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
        let email_contents: Box<dyn EmailData + Send + 'static> = if let Some(true) = is_token_only
        {
            Box::new(email_types::InviteRegisteredUser {
                recipient_email: invitee_email,
                user_name: domain::UserName::new(new_user.get_name())?,
                settings: state.conf.clone(),
                subject: "You have been invited to join Hyperswitch Community!",
                merchant_id: user_from_token.merchant_id.clone(),
                auth_id: auth_id.clone(),
            })
        } else {
            Box::new(email_types::InviteUser {
                recipient_email: invitee_email,
                user_name: domain::UserName::new(new_user.get_name())?,
                settings: state.conf.clone(),
                subject: "You have been invited to join Hyperswitch Community!",
                merchant_id: user_from_token.merchant_id.clone(),
                auth_id: auth_id.clone(),
            })
        };
        let send_email_result = state
            .email_client
            .compose_and_send_email(email_contents, state.conf.proxy.https_url.as_ref())
            .await;
        logger::info!(?send_email_result);
        is_email_sent = send_email_result.is_ok();
    }
    #[cfg(not(feature = "email"))]
    {
        is_email_sent = false;

        let invited_user_token = auth::UserFromToken {
            user_id: new_user.get_user_id(),
            merchant_id: user_from_token.merchant_id.clone(),
            org_id: user_from_token.org_id.clone(),
            role_id: request.role_id.clone(),
        };

        let set_metadata_request = SetMetaDataRequest::IsChangePasswordRequired;
        dashboard_metadata::set_metadata(
            state.clone(),
            invited_user_token,
            set_metadata_request,
            req_state,
        )
        .await?;
    }

    Ok(InviteMultipleUserResponse {
        is_email_sent,
        password: new_user
            .get_password()
            .map(|password| password.get_secret()),
        email: request.email.clone(),
        error: None,
    })
}

#[cfg(feature = "email")]
pub async fn resend_invite(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: user_api::ReInviteUserRequest,
    auth_id: Option<String>,
) -> UserResponse<()> {
    let invitee_email = domain::UserEmail::from_pii_email(request.email)?;
    let user: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(&invitee_email.clone().into_inner())
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::InvalidRoleOperation)
                    .attach_printable("User not found in the records")
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?
        .into();
    let user_role = state
        .store
        .find_user_role_by_user_id_merchant_id(user.get_user_id(), &user_from_token.merchant_id)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::InvalidRoleOperation)
                    .attach_printable(format!(
                        "User role with user_id = {} and merchant_id = {} is not found",
                        user.get_user_id(),
                        user_from_token.merchant_id
                    ))
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    if !matches!(user_role.status, UserStatus::InvitationSent) {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("User status is not InvitationSent".to_string());
    }

    let email_contents = email_types::InviteUser {
        recipient_email: invitee_email,
        user_name: domain::UserName::new(user.get_name())?,
        settings: state.conf.clone(),
        subject: "You have been invited to join Hyperswitch Community!",
        merchant_id: user_from_token.merchant_id,
        auth_id,
    };
    state
        .email_client
        .compose_and_send_email(
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "email")]
pub async fn accept_invite_from_email(
    state: SessionState,
    request: user_api::AcceptInviteFromEmailRequest,
) -> UserResponse<user_api::DashboardEntryResponse> {
    let token = request.token.expose();

    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(
            &email_token
                .get_email()
                .change_context(UserErrors::InternalServerError)?,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let merchant_id = email_token
        .get_merchant_id()
        .ok_or(UserErrors::InternalServerError)?;

    let update_status_result = state
        .store
        .update_user_role_by_user_id_merchant_id(
            user.get_user_id(),
            merchant_id,
            UserRoleUpdate::UpdateStatus {
                status: UserStatus::Active,
                modified_by: user.get_user_id().to_string(),
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|e| logger::error!(?e));

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .update_user_by_user_id(user.get_user_id(), storage_user::UserUpdate::VerifyUser)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let token =
        utils::user::generate_jwt_auth_token(&state, &user_from_db, &update_status_result).await?;
    utils::user_role::set_role_permissions_in_cache_by_user_role(&state, &update_status_result)
        .await;

    let response = utils::user::get_dashboard_entry_response(
        &state,
        user_from_db,
        update_status_result,
        token.clone(),
    )?;

    auth::cookies::set_cookie_response(response, token)
}

#[cfg(feature = "email")]
pub async fn accept_invite_from_email_token_only_flow(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    request: user_api::AcceptInviteFromEmailRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::DashboardEntryResponse>> {
    let token = request.token.expose();

    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(
            &email_token
                .get_email()
                .change_context(UserErrors::InternalServerError)?,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_user_id() != user_token.user_id {
        return Err(UserErrors::LinkInvalid.into());
    }

    let merchant_id = email_token
        .get_merchant_id()
        .ok_or(UserErrors::LinkInvalid)?;

    let user_role = state
        .store
        .update_user_role_by_user_id_merchant_id(
            user_from_db.get_user_id(),
            merchant_id,
            UserRoleUpdate::UpdateStatus {
                status: UserStatus::Active,
                modified_by: user_from_db.get_user_id().to_string(),
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    if !user_from_db.is_verified() {
        let _ = state
            .global_store
            .update_user_by_user_id(
                user_from_db.get_user_id(),
                storage_user::UserUpdate::VerifyUser,
            )
            .await
            .map_err(|e| logger::error!(?e));
    }

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|e| logger::error!(?e));

    let current_flow = domain::CurrentFlow::new(
        user_token,
        domain::SPTFlow::AcceptInvitationFromEmail.into(),
    )?;
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

pub async fn create_internal_user(
    state: SessionState,
    request: user_api::CreateInternalUserRequest,
) -> UserResponse<()> {
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

    let internal_merchant = state
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

    let new_user = domain::NewUser::try_from((request, internal_merchant.organization_id))?;

    let mut store_user: storage_user::UserNew = new_user.clone().try_into()?;
    store_user.set_is_verified(true);

    state
        .global_store
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
    state: SessionState,
    request: user_api::SwitchMerchantIdRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::DashboardEntryResponse> {
    if user_from_token.merchant_id == request.merchant_id {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User switching to same merchant id".to_string(),
        )
        .into());
    }

    let user = user_from_token.get_user_from_db(&state).await?;

    let role_info = roles::RoleInfo::from_role_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .to_not_found_response(UserErrors::InternalServerError)?;

    let (token, role_id) = if role_info.is_internal() {
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

        let token = utils::user::generate_jwt_auth_token_with_custom_role_attributes(
            &state,
            &user,
            request.merchant_id.clone(),
            org_id.clone(),
            user_from_token.role_id.clone(),
        )
        .await?;

        (token, user_from_token.role_id)
    } else {
        let user_roles = state
            .store
            .list_user_roles_by_user_id(&user_from_token.user_id)
            .await
            .change_context(UserErrors::InternalServerError)?;

        let active_user_roles = user_roles
            .into_iter()
            .filter(|role| role.status == UserStatus::Active)
            .collect::<Vec<_>>();

        let user_role = active_user_roles
            .iter()
            .find(|role| role.merchant_id == request.merchant_id)
            .ok_or(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("User doesn't have access to switch")?;

        let token = utils::user::generate_jwt_auth_token(&state, &user, user_role).await?;
        utils::user_role::set_role_permissions_in_cache_by_user_role(&state, user_role).await;

        (token, user_role.role_id.clone())
    };

    let response = user_api::DashboardEntryResponse {
        token: token.clone(),
        name: user.get_name(),
        email: user.get_email(),
        user_id: user.get_user_id().to_string(),
        verification_days_left: None,
        user_role: role_id,
        merchant_id: request.merchant_id,
    };

    auth::cookies::set_cookie_response(response, token)
}

pub async fn create_merchant_account(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: user_api::UserMerchantCreate,
) -> UserResponse<()> {
    let user_from_db = user_from_token.get_user_from_db(&state).await?;

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

pub async fn list_merchants_for_user(
    state: SessionState,
    user_from_token: auth::UserIdFromAuth,
) -> UserResponse<Vec<user_api::UserMerchantAccount>> {
    let user_roles = state
        .store
        .list_user_roles_by_user_id(user_from_token.user_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)?;

    let merchant_accounts = state
        .store
        .list_multiple_merchant_accounts(
            user_roles
                .iter()
                .map(|role| role.merchant_id.clone())
                .collect(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    let roles =
        utils::user_role::get_multiple_role_info_for_user_roles(&state, &user_roles).await?;

    Ok(ApplicationResponse::Json(
        utils::user::get_multiple_merchant_details_with_status(
            user_roles,
            merchant_accounts,
            roles,
        )?,
    ))
}

pub async fn get_user_details_in_merchant_account(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: user_api::GetUserRoleDetailsRequest,
    _req_state: ReqState,
) -> UserResponse<user_api::GetUserRoleDetailsResponse> {
    let required_user = utils::user::get_user_from_db_by_email(&state, request.email.try_into()?)
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)?;

    let required_user_role = state
        .store
        .find_user_role_by_user_id_merchant_id(
            required_user.get_user_id(),
            &user_from_token.merchant_id,
        )
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)
        .attach_printable("User not found in the merchant account")?;

    let role_info = roles::RoleInfo::from_role_id(
        &state,
        &required_user_role.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("User role exists but the corresponding role doesn't")?;

    Ok(ApplicationResponse::Json(
        user_api::GetUserRoleDetailsResponse {
            email: required_user.get_email(),
            name: required_user.get_name(),
            role_id: role_info.get_role_id().to_string(),
            role_name: role_info.get_role_name().to_string(),
            status: required_user_role.status.foreign_into(),
            last_modified_at: required_user_role.last_modified,
            groups: role_info.get_permission_groups().to_vec(),
            role_scope: role_info.get_scope(),
        },
    ))
}

pub async fn list_users_for_merchant_account(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::ListUsersResponse> {
    let user_roles: HashMap<String, _> = state
        .store
        .list_user_roles_by_merchant_id(user_from_token.merchant_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("No user roles for given merchant id")?
        .into_iter()
        .map(|role| (role.user_id.clone(), role))
        .collect();

    let user_ids = user_roles.keys().cloned().collect::<Vec<_>>();

    let users = state
        .global_store
        .find_users_by_user_ids(user_ids)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("No users for given merchant id")?;

    let users_and_user_roles: Vec<_> = users
        .into_iter()
        .filter_map(|user| {
            user_roles
                .get(&user.user_id)
                .map(|role| (user.clone(), role.clone()))
        })
        .collect();

    let users_user_roles_and_roles =
        futures::future::try_join_all(users_and_user_roles.into_iter().map(
            |(user, user_role)| async {
                roles::RoleInfo::from_role_id(
                    &state,
                    &user_role.role_id.clone(),
                    &user_role.merchant_id,
                    &user_role.org_id,
                )
                .await
                .map(|role_info| (user, user_role, role_info))
                .to_not_found_response(UserErrors::InternalServerError)
            },
        ))
        .await?;

    let user_details_vec = users_user_roles_and_roles
        .into_iter()
        .map(|(user, user_role, role_info)| {
            let user = domain::UserFromStorage::from(user);
            user_api::UserDetails {
                email: user.get_email(),
                name: user.get_name(),
                role_id: user_role.role_id.clone(),
                role_name: role_info.get_role_name().to_string(),
                status: user_role.status.foreign_into(),
                last_modified_at: user_role.last_modified,
            }
        })
        .collect();

    Ok(ApplicationResponse::Json(user_api::ListUsersResponse(
        user_details_vec,
    )))
}

#[cfg(feature = "email")]
pub async fn verify_email(
    state: SessionState,
    req: user_api::VerifyEmailRequest,
) -> UserResponse<user_api::SignInResponse> {
    let token = req.token.clone().expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user = state
        .global_store
        .find_user_by_email(
            &email_token
                .get_email()
                .change_context(UserErrors::InternalServerError)?,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    let user = state
        .global_store
        .update_user_by_user_id(user.user_id.as_str(), storage_user::UserUpdate::VerifyUser)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let user_from_db: domain::UserFromStorage = user.into();

    let signin_strategy =
        if let Some(preferred_merchant_id) = user_from_db.get_preferred_merchant_id() {
            let preferred_role = user_from_db
                .get_role_from_db_by_merchant_id(&state, preferred_merchant_id.as_str())
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("User role with preferred_merchant_id not found")?;
            domain::SignInWithRoleStrategyType::SingleRole(domain::SignInWithSingleRoleStrategy {
                user: user_from_db,
                user_role: preferred_role,
            })
        } else {
            let user_roles = user_from_db.get_roles_from_db(&state).await?;
            domain::SignInWithRoleStrategyType::decide_signin_strategy_by_user_roles(
                user_from_db,
                user_roles,
            )
            .await?
        };

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|e| logger::error!(?e));

    let response = signin_strategy.get_signin_response(&state).await?;
    let token = utils::user::get_token_from_signin_response(&response);
    auth::cookies::set_cookie_response(response, token)
}

#[cfg(feature = "email")]
pub async fn verify_email_token_only_flow(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_api::VerifyEmailRequest,
) -> UserResponse<user_api::TokenOrPayloadResponse<user_api::SignInResponse>> {
    let token = req.token.clone().expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user_from_email = state
        .global_store
        .find_user_by_email(
            &email_token
                .get_email()
                .change_context(UserErrors::InternalServerError)?,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    if user_from_email.user_id != user_token.user_id {
        return Err(UserErrors::LinkInvalid.into());
    }

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .update_user_by_user_id(
            user_from_email.user_id.as_str(),
            storage_user::UserUpdate::VerifyUser,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|e| logger::error!(?e));

    let current_flow = domain::CurrentFlow::new(user_token, domain::SPTFlow::VerifyEmail.into())?;
    let next_flow = current_flow.next(user_from_db, &state).await?;
    let token = next_flow.get_token(&state).await?;

    let response = user_api::TokenOrPayloadResponse::Token(user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    });

    auth::cookies::set_cookie_response(response, token)
}

#[cfg(feature = "email")]
pub async fn send_verification_mail(
    state: SessionState,
    req: user_api::SendVerifyEmailRequest,
    auth_id: Option<String>,
) -> UserResponse<()> {
    let user_email = domain::UserEmail::try_from(req.email)?;
    let user = state
        .global_store
        .find_user_by_email(&user_email.into_inner())
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::UserNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    if user.is_verified {
        return Err(UserErrors::UserAlreadyVerified.into());
    }

    let email_contents = email_types::VerifyEmail {
        recipient_email: domain::UserEmail::from_pii_email(user.email)?,
        settings: state.conf.clone(),
        subject: "Welcome to the Hyperswitch community!",
        auth_id,
    };

    state
        .email_client
        .compose_and_send_email(
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "recon")]
pub async fn verify_token(
    state: SessionState,
    req: auth::ReconUser,
) -> UserResponse<user_api::VerifyTokenResponse> {
    let user = state
        .global_store
        .find_user_by_id(&req.user_id)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::UserNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;
    let merchant_id = state
        .store
        .find_user_role_by_user_id(&req.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .merchant_id;

    Ok(ApplicationResponse::Json(user_api::VerifyTokenResponse {
        merchant_id: merchant_id.to_string(),
        user_email: user.email,
    }))
}

pub async fn update_user_details(
    state: SessionState,
    user_token: auth::UserFromToken,
    req: user_api::UpdateUserAccountDetailsRequest,
    _req_state: ReqState,
) -> UserResponse<()> {
    let user: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let name = req.name.map(domain::UserName::new).transpose()?;

    if let Some(ref preferred_merchant_id) = req.preferred_merchant_id {
        let _ = state
            .store
            .find_user_role_by_user_id_merchant_id(user.get_user_id(), preferred_merchant_id)
            .await
            .map_err(|e| {
                if e.current_context().is_db_not_found() {
                    e.change_context(UserErrors::MerchantIdNotFound)
                } else {
                    e.change_context(UserErrors::InternalServerError)
                }
            })?;
    }

    let user_update = storage_user::UserUpdate::AccountUpdate {
        name: name.map(|x| x.get_secret().expose()),
        is_verified: None,
        preferred_merchant_id: req.preferred_merchant_id,
    };

    state
        .global_store
        .update_user_by_user_id(user.get_user_id(), user_update)
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "email")]
pub async fn user_from_email(
    state: SessionState,
    req: user_api::UserFromEmailRequest,
) -> UserResponse<user_api::TokenResponse> {
    let token = req.token.expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(
            &email_token
                .get_email()
                .change_context(UserErrors::InternalServerError)?,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let next_flow =
        domain::NextFlow::from_origin(email_token.get_flow(), user_from_db.clone(), &state).await?;

    let token = next_flow.get_token(&state).await?;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    };
    auth::cookies::set_cookie_response(response, token)
}

pub async fn begin_totp(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
) -> UserResponse<user_api::BeginTotpResponse> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_totp_status() == TotpStatus::Set {
        return Ok(ApplicationResponse::Json(user_api::BeginTotpResponse {
            secret: None,
        }));
    }

    let totp = tfa_utils::generate_default_totp(
        user_from_db.get_email(),
        None,
        state.conf.user.totp_issuer_name.clone(),
    )?;
    let secret = totp.get_secret_base32().into();
    tfa_utils::insert_totp_secret_in_redis(&state, &user_token.user_id, &secret).await?;

    Ok(ApplicationResponse::Json(user_api::BeginTotpResponse {
        secret: Some(user_api::TotpSecret {
            secret,
            totp_url: totp.get_url().into(),
        }),
    }))
}

pub async fn reset_totp(
    state: SessionState,
    user_token: auth::UserFromToken,
) -> UserResponse<user_api::BeginTotpResponse> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_totp_status() != TotpStatus::Set {
        return Err(UserErrors::TotpNotSetup.into());
    }

    if !tfa_utils::check_totp_in_redis(&state, &user_token.user_id).await?
        && !tfa_utils::check_recovery_code_in_redis(&state, &user_token.user_id).await?
    {
        return Err(UserErrors::TwoFactorAuthRequired.into());
    }

    let totp = tfa_utils::generate_default_totp(
        user_from_db.get_email(),
        None,
        state.conf.user.totp_issuer_name.clone(),
    )?;

    let secret = totp.get_secret_base32().into();
    tfa_utils::insert_totp_secret_in_redis(&state, &user_token.user_id, &secret).await?;

    Ok(ApplicationResponse::Json(user_api::BeginTotpResponse {
        secret: Some(user_api::TotpSecret {
            secret,
            totp_url: totp.get_url().into(),
        }),
    }))
}

pub async fn verify_totp(
    state: SessionState,
    user_token: auth::UserIdFromAuth,
    req: user_api::VerifyTotpRequest,
) -> UserResponse<user_api::TokenResponse> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_totp_status() != TotpStatus::Set {
        return Err(UserErrors::TotpNotSetup.into());
    }

    let user_totp_secret = user_from_db
        .decrypt_and_get_totp_secret(&state)
        .await?
        .ok_or(UserErrors::InternalServerError)?;

    let totp = tfa_utils::generate_default_totp(
        user_from_db.get_email(),
        Some(user_totp_secret),
        state.conf.user.totp_issuer_name.clone(),
    )?;

    if totp
        .generate_current()
        .change_context(UserErrors::InternalServerError)?
        != req.totp.expose()
    {
        return Err(UserErrors::InvalidTotp.into());
    }

    tfa_utils::insert_totp_in_redis(&state, &user_token.user_id).await?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn update_totp(
    state: SessionState,
    user_token: auth::UserIdFromAuth,
    req: user_api::VerifyTotpRequest,
) -> UserResponse<()> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let new_totp_secret = tfa_utils::get_totp_secret_from_redis(&state, &user_token.user_id)
        .await?
        .ok_or(UserErrors::TotpSecretNotFound)?;

    let totp = tfa_utils::generate_default_totp(
        user_from_db.get_email(),
        Some(new_totp_secret),
        state.conf.user.totp_issuer_name.clone(),
    )?;

    if totp
        .generate_current()
        .change_context(UserErrors::InternalServerError)?
        != req.totp.expose()
    {
        return Err(UserErrors::InvalidTotp.into());
    }

    let key_store = user_from_db.get_or_create_key_store(&state).await?;

    state
        .global_store
        .update_user_by_user_id(
            &user_token.user_id,
            storage_user::UserUpdate::TotpUpdate {
                totp_status: None,
                totp_secret: Some(
                    // TODO: Impl conversion trait for User and move this there
                    domain::types::encrypt::<String, masking::WithType>(
                        totp.get_secret_base32().into(),
                        key_store.key.peek(),
                    )
                    .await
                    .change_context(UserErrors::InternalServerError)?
                    .into(),
                ),

                totp_recovery_codes: None,
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    let _ = tfa_utils::delete_totp_secret_from_redis(&state, &user_token.user_id)
        .await
        .map_err(|e| logger::error!(?e));

    // This is not the main task of this API, so we don't throw error if this fails.
    // Any following API which requires TOTP will throw error if TOTP is not set in redis
    // and FE will ask user to enter TOTP again
    let _ = tfa_utils::insert_totp_in_redis(&state, &user_token.user_id)
        .await
        .map_err(|e| logger::error!(?e));

    Ok(ApplicationResponse::StatusOk)
}

pub async fn generate_recovery_codes(
    state: SessionState,
    user_token: auth::UserIdFromAuth,
) -> UserResponse<user_api::RecoveryCodes> {
    if !tfa_utils::check_totp_in_redis(&state, &user_token.user_id).await? {
        return Err(UserErrors::TotpRequired.into());
    }

    let recovery_codes = domain::RecoveryCodes::generate_new();

    state
        .global_store
        .update_user_by_user_id(
            &user_token.user_id,
            storage_user::UserUpdate::TotpUpdate {
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: Some(
                    recovery_codes
                        .get_hashed()
                        .change_context(UserErrors::InternalServerError)?,
                ),
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(user_api::RecoveryCodes {
        recovery_codes: recovery_codes.into_inner(),
    }))
}

pub async fn verify_recovery_code(
    state: SessionState,
    user_token: auth::UserIdFromAuth,
    req: user_api::VerifyRecoveryCodeRequest,
) -> UserResponse<user_api::TokenResponse> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_totp_status() != TotpStatus::Set {
        return Err(UserErrors::TwoFactorAuthNotSetup.into());
    }

    let mut recovery_codes = user_from_db
        .get_recovery_codes()
        .ok_or(UserErrors::InternalServerError)?;

    let matching_index = utils::user::password::get_index_for_correct_recovery_code(
        &req.recovery_code,
        &recovery_codes,
    )?
    .ok_or(UserErrors::InvalidRecoveryCode)?;

    tfa_utils::insert_recovery_code_in_redis(&state, user_from_db.get_user_id()).await?;
    let _ = recovery_codes.remove(matching_index);

    state
        .global_store
        .update_user_by_user_id(
            user_from_db.get_user_id(),
            storage_user::UserUpdate::TotpUpdate {
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: Some(recovery_codes),
            },
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn terminate_two_factor_auth(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    skip_two_factor_auth: bool,
) -> UserResponse<user_api::TokenResponse> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if !skip_two_factor_auth {
        if !tfa_utils::check_totp_in_redis(&state, &user_token.user_id).await?
            && !tfa_utils::check_recovery_code_in_redis(&state, &user_token.user_id).await?
        {
            return Err(UserErrors::TwoFactorAuthRequired.into());
        }

        if user_from_db.get_recovery_codes().is_none() {
            return Err(UserErrors::TwoFactorAuthNotSetup.into());
        }

        if user_from_db.get_totp_status() != TotpStatus::Set {
            state
                .global_store
                .update_user_by_user_id(
                    user_from_db.get_user_id(),
                    storage_user::UserUpdate::TotpUpdate {
                        totp_status: Some(TotpStatus::Set),
                        totp_secret: None,
                        totp_recovery_codes: None,
                    },
                )
                .await
                .change_context(UserErrors::InternalServerError)?;
        }
    }

    let current_flow = domain::CurrentFlow::new(user_token, domain::SPTFlow::TOTP.into())?;
    let next_flow = current_flow.next(user_from_db, &state).await?;
    let token = next_flow.get_token(&state).await?;

    auth::cookies::set_cookie_response(
        user_api::TokenResponse {
            token: token.clone(),
            token_type: next_flow.get_flow().into(),
        },
        token,
    )
}

pub async fn check_two_factor_auth_status(
    state: SessionState,
    user_token: auth::UserFromToken,
) -> UserResponse<user_api::TwoFactorAuthStatusResponse> {
    Ok(ApplicationResponse::Json(
        user_api::TwoFactorAuthStatusResponse {
            totp: tfa_utils::check_totp_in_redis(&state, &user_token.user_id).await?,
            recovery_code: tfa_utils::check_recovery_code_in_redis(&state, &user_token.user_id)
                .await?,
        },
    ))
}

pub async fn create_user_authentication_method(
    state: SessionState,
    req: user_api::CreateUserAuthenticationMethodRequest,
) -> UserResponse<()> {
    let user_auth_encryption_key = hex::decode(
        state
            .conf
            .user_auth_methods
            .get_inner()
            .encryption_key
            .clone()
            .expose(),
    )
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to decode DEK")?;

    let (private_config, public_config) = match req.auth_method {
        user_api::AuthConfig::OpenIdConnect {
            ref private_config,
            ref public_config,
        } => {
            let private_config_value = serde_json::to_value(private_config.clone())
                .change_context(UserErrors::AuthConfigParsingError)
                .attach_printable("Failed to convert auth config to json")?;

            let encrypted_config = domain::types::encrypt::<serde_json::Value, masking::WithType>(
                private_config_value.into(),
                &user_auth_encryption_key,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to encrypt auth config")?;

            Ok::<_, error_stack::Report<UserErrors>>((
                Some(encrypted_config.into()),
                Some(
                    serde_json::to_value(public_config.clone())
                        .change_context(UserErrors::AuthConfigParsingError)
                        .attach_printable("Failed to convert auth config to json")?,
                ),
            ))
        }
        _ => Ok((None, None)),
    }?;

    let auth_methods = state
        .store
        .list_user_authentication_methods_for_owner_id(&req.owner_id)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get list of auth methods for the owner id")?;

    let auth_id = auth_methods
        .first()
        .map(|auth_method| auth_method.auth_id.clone())
        .unwrap_or(uuid::Uuid::new_v4().to_string());

    let now = common_utils::date_time::now();
    state
        .store
        .insert_user_authentication_method(UserAuthenticationMethodNew {
            id: uuid::Uuid::new_v4().to_string(),
            auth_id,
            owner_id: req.owner_id,
            owner_type: req.owner_type,
            auth_type: req.auth_method.foreign_into(),
            private_config,
            public_config,
            allow_signup: req.allow_signup,
            created_at: now,
            last_modified_at: now,
        })
        .await
        .to_duplicate_response(UserErrors::UserAuthMethodAlreadyExists)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn update_user_authentication_method(
    state: SessionState,
    req: user_api::UpdateUserAuthenticationMethodRequest,
) -> UserResponse<()> {
    let user_auth_encryption_key = hex::decode(
        state
            .conf
            .user_auth_methods
            .get_inner()
            .encryption_key
            .clone()
            .expose(),
    )
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to decode DEK")?;

    let (private_config, public_config) = match req.auth_method {
        user_api::AuthConfig::OpenIdConnect {
            ref private_config,
            ref public_config,
        } => {
            let private_config_value = serde_json::to_value(private_config.clone())
                .change_context(UserErrors::AuthConfigParsingError)
                .attach_printable("Failed to convert auth config to json")?;

            let encrypted_config = domain::types::encrypt::<serde_json::Value, masking::WithType>(
                private_config_value.into(),
                &user_auth_encryption_key,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to encrypt auth config")?;

            Ok::<_, error_stack::Report<UserErrors>>((
                Some(encrypted_config.into()),
                Some(
                    serde_json::to_value(public_config.clone())
                        .change_context(UserErrors::AuthConfigParsingError)
                        .attach_printable("Failed to convert auth config to json")?,
                ),
            ))
        }
        _ => Ok((None, None)),
    }?;

    state
        .store
        .update_user_authentication_method(
            &req.id,
            UserAuthenticationMethodUpdate::UpdateConfig {
                private_config,
                public_config,
            },
        )
        .await
        .change_context(UserErrors::InvalidUserAuthMethodOperation)?;
    Ok(ApplicationResponse::StatusOk)
}

pub async fn list_user_authentication_methods(
    state: SessionState,
    req: user_api::GetUserAuthenticationMethodsRequest,
) -> UserResponse<Vec<user_api::UserAuthenticationMethodResponse>> {
    let user_authentication_methods = state
        .store
        .list_user_authentication_methods_for_auth_id(&req.auth_id)
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(
        user_authentication_methods
            .into_iter()
            .map(|auth_method| {
                let auth_name = match (auth_method.auth_type, auth_method.public_config) {
                    (common_enums::UserAuthType::OpenIdConnect, Some(config)) => {
                        let open_id_public_config: user_api::OpenIdConnectPublicConfig = config
                            .parse_value("OpenIdConnectPublicConfig")
                            .change_context(UserErrors::InternalServerError)
                            .attach_printable("unable to parse generic data value")?;

                        Ok(Some(open_id_public_config.name))
                    }
                    (common_enums::UserAuthType::OpenIdConnect, None) => {
                        Err(UserErrors::InternalServerError)
                            .attach_printable("No config found for open_id_connect auth_method")
                    }
                    _ => Ok(None),
                }?;

                Ok(user_api::UserAuthenticationMethodResponse {
                    id: auth_method.id,
                    auth_id: auth_method.auth_id,
                    auth_method: user_api::AuthMethodDetails {
                        name: auth_name,
                        auth_type: auth_method.auth_type,
                    },
                    allow_signup: auth_method.allow_signup,
                })
            })
            .collect::<UserResult<_>>()?,
    ))
}

pub async fn get_sso_auth_url(
    state: SessionState,
    request: user_api::GetSsoAuthUrlRequest,
) -> UserResponse<()> {
    let user_authentication_method = state
        .store
        .get_user_authentication_method_by_id(request.id.as_str())
        .await
        .to_not_found_response(UserErrors::InvalidUserAuthMethodOperation)?;

    let open_id_private_config =
        utils::user::decrypt_oidc_private_config(&state, user_authentication_method.private_config)
            .await?;

    let open_id_public_config: user_api::OpenIdConnectPublicConfig = user_authentication_method
        .public_config
        .ok_or(UserErrors::InternalServerError)
        .attach_printable("Public config not present")?
        .parse_value("OpenIdConnectPublicConfig")
        .change_context(UserErrors::InternalServerError)
        .attach_printable("unable to parse OpenIdConnectPublicConfig")?;

    let oidc_state = Secret::new(nanoid::nanoid!());
    utils::user::set_sso_id_in_redis(&state, oidc_state.clone(), request.id).await?;

    let redirect_url =
        utils::user::get_oidc_sso_redirect_url(&state, &open_id_public_config.name.to_string());

    openidconnect::get_authorization_url(
        state,
        redirect_url,
        oidc_state,
        open_id_private_config.base_url.into(),
        open_id_private_config.client_id,
    )
    .await
    .map(|url| {
        ApplicationResponse::JsonForRedirection(RedirectionResponse {
            headers: Vec::with_capacity(0),
            return_url: String::new(),
            http_method: String::new(),
            params: Vec::with_capacity(0),
            return_url_with_query_params: url.to_string(),
        })
    })
}

pub async fn sso_sign(
    state: SessionState,
    request: user_api::SsoSignInRequest,
    user_from_single_purpose_token: Option<auth::UserFromSinglePurposeToken>,
) -> UserResponse<user_api::TokenResponse> {
    let authentication_method_id =
        utils::user::get_sso_id_from_redis(&state, request.state.clone()).await?;

    let user_authentication_method = state
        .store
        .get_user_authentication_method_by_id(&authentication_method_id)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let open_id_private_config =
        utils::user::decrypt_oidc_private_config(&state, user_authentication_method.private_config)
            .await?;

    let open_id_public_config: user_api::OpenIdConnectPublicConfig = user_authentication_method
        .public_config
        .ok_or(UserErrors::InternalServerError)
        .attach_printable("Public config not present")?
        .parse_value("OpenIdConnectPublicConfig")
        .change_context(UserErrors::InternalServerError)
        .attach_printable("unable to parse OpenIdConnectPublicConfig")?;

    let redirect_url =
        utils::user::get_oidc_sso_redirect_url(&state, &open_id_public_config.name.to_string());
    let email = openidconnect::get_user_email_from_oidc_provider(
        &state,
        redirect_url,
        request.state,
        open_id_private_config.base_url.into(),
        open_id_private_config.client_id,
        request.code,
        open_id_private_config.client_secret,
    )
    .await?;

    // TODO: Use config to handle not found error
    let user_from_db = state
        .global_store
        .find_user_by_email(&email.into_inner())
        .await
        .map(Into::into)
        .to_not_found_response(UserErrors::UserNotFound)?;

    let next_flow = if let Some(user_from_single_purpose_token) = user_from_single_purpose_token {
        let current_flow =
            domain::CurrentFlow::new(user_from_single_purpose_token, domain::SPTFlow::SSO.into())?;
        current_flow.next(user_from_db, &state).await?
    } else {
        domain::NextFlow::from_origin(domain::Origin::SignInWithSSO, user_from_db, &state).await?
    };

    let token = next_flow.get_token(&state).await?;
    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    };

    auth::cookies::set_cookie_response(response, token)
}
