use api_models::user::{self as user_api, InviteMultipleUserResponse};
#[cfg(feature = "email")]
use diesel_models::user_role::UserRoleUpdate;
use diesel_models::{enums::UserStatus, user as storage_user, user_role::UserRoleNew};
#[cfg(feature = "email")]
use error_stack::IntoReport;
use error_stack::ResultExt;
use masking::ExposeInterface;
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
    routes::AppState,
    services::{authentication as auth, authorization::roles, ApplicationResponse},
    types::{domain, transformers::ForeignInto},
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
    let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;

    Ok(ApplicationResponse::Json(
        utils::user::get_dashboard_entry_response(&state, user_from_db, user_role, token)?,
    ))
}

pub async fn signin_without_invite_checks(
    state: AppState,
    request: user_api::SignInRequest,
) -> UserResponse<user_api::DashboardEntryResponse> {
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
    let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;

    Ok(ApplicationResponse::Json(
        utils::user::get_dashboard_entry_response(&state, user_from_db, user_role, token)?,
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

    Ok(ApplicationResponse::Json(
        signin_strategy.get_signin_response(&state).await?,
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

pub async fn signout(state: AppState, user_from_token: auth::UserFromToken) -> UserResponse<()> {
    auth::blacklist::insert_user_in_blacklist(&state, &user_from_token.user_id).await?;
    Ok(ApplicationResponse::StatusOk)
}

pub async fn change_password(
    state: AppState,
    request: user_api::ChangePasswordRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<()> {
    let user: domain::UserFromStorage = state
        .store
        .find_user_by_id(&user_from_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    user.compare_password(request.old_password.to_owned())
        .change_context(UserErrors::InvalidOldPassword)?;

    if request.old_password == request.new_password {
        return Err(UserErrors::ChangePasswordError.into());
    }
    let new_password = domain::UserPassword::new(request.new_password)?;

    let new_password_hash =
        utils::user::password::generate_password_hash(new_password.get_secret())?;

    let _ = state
        .store
        .update_user_by_user_id(
            user.get_user_id(),
            diesel_models::user::UserUpdate::AccountUpdate {
                name: None,
                password: Some(new_password_hash),
                is_verified: None,
                preferred_merchant_id: None,
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
    state: AppState,
    request: user_api::ForgotPasswordRequest,
) -> UserResponse<()> {
    let user_email = domain::UserEmail::from_pii_email(request.email)?;

    let user_from_db = state
        .store
        .find_user_by_email(user_email.get_secret().expose().as_str())
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

#[cfg(feature = "email")]
pub async fn reset_password(
    state: AppState,
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
        .store
        .update_user_by_email(
            email_token.get_email(),
            storage_user::UserUpdate::AccountUpdate {
                name: None,
                password: Some(hash_password),
                is_verified: Some(true),
                preferred_merchant_id: None,
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

pub async fn invite_user(
    state: AppState,
    request: user_api::InviteUserRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::InviteUserResponse> {
    let inviter_user = state
        .store
        .find_user_by_id(user_from_token.user_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)?;

    if inviter_user.email == request.email {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User Inviting themselves".to_string(),
        )
        .into());
    }

    let role_info = roles::get_role_info_from_role_id(
        &state,
        request.role_id.as_str(),
        &user_from_token.merchant_id,
        &user_from_token.org_id,
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    if !role_info.is_invitable() {
        return Err(UserErrors::InvalidRoleId.into())
            .attach_printable(format!("role_id = {} is not invitable", request.role_id));
    }

    let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;

    let invitee_user = state
        .store
        .find_user_by_email(invitee_email.clone().get_secret().expose().as_str())
        .await;

    if let Ok(invitee_user) = invitee_user {
        let invitee_user_from_db = domain::UserFromStorage::from(invitee_user);

        let now = common_utils::date_time::now();
        state
            .store
            .insert_user_role(UserRoleNew {
                user_id: invitee_user_from_db.get_user_id().to_owned(),
                merchant_id: user_from_token.merchant_id,
                role_id: request.role_id,
                org_id: user_from_token.org_id,
                status: {
                    if cfg!(feature = "email") {
                        UserStatus::InvitationSent
                    } else {
                        UserStatus::Active
                    }
                },
                created_by: user_from_token.user_id.clone(),
                last_modified_by: user_from_token.user_id,
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

        Ok(ApplicationResponse::Json(user_api::InviteUserResponse {
            is_email_sent: false,
            password: None,
        }))
    } else if invitee_user
        .as_ref()
        .map_err(|e| e.current_context().is_db_not_found())
        .err()
        .unwrap_or(false)
    {
        let new_user = domain::NewUser::try_from((request.clone(), user_from_token.clone()))?;

        new_user
            .insert_user_in_db(state.store.as_ref())
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
                last_modified_by: user_from_token.user_id,
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
            let email_contents = email_types::InviteUser {
                recipient_email: invitee_email,
                user_name: domain::UserName::new(new_user.get_name())?,
                settings: state.conf.clone(),
                subject: "You have been invited to join Hyperswitch Community!",
                merchant_id: user_from_token.merchant_id,
            };
            let send_email_result = state
                .email_client
                .compose_and_send_email(
                    Box::new(email_contents),
                    state.conf.proxy.https_url.as_ref(),
                )
                .await;
            logger::info!(?send_email_result);
            is_email_sent = send_email_result.is_ok();
        }
        #[cfg(not(feature = "email"))]
        {
            is_email_sent = false;
            let invited_user_token = auth::UserFromToken {
                user_id: new_user.get_user_id(),
                merchant_id: user_from_token.merchant_id,
                org_id: user_from_token.org_id,
                role_id: request.role_id,
            };

            let set_metadata_request = SetMetaDataRequest::IsChangePasswordRequired;
            dashboard_metadata::set_metadata(
                state.clone(),
                invited_user_token,
                set_metadata_request,
            )
            .await?;
        }

        Ok(ApplicationResponse::Json(user_api::InviteUserResponse {
            is_email_sent,
            password: if cfg!(not(feature = "email")) {
                Some(new_user.get_password().get_secret())
            } else {
                None
            },
        }))
    } else {
        Err(UserErrors::InternalServerError.into())
    }
}

pub async fn invite_multiple_user(
    state: AppState,
    user_from_token: auth::UserFromToken,
    requests: Vec<user_api::InviteUserRequest>,
) -> UserResponse<Vec<InviteMultipleUserResponse>> {
    if requests.len() > 10 {
        return Err(UserErrors::MaxInvitationsError.into())
            .attach_printable("Number of invite requests must not exceed 10");
    }

    let responses = futures::future::join_all(requests.iter().map(|request| async {
        match handle_invitation(&state, &user_from_token, request).await {
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
    state: &AppState,
    user_from_token: &auth::UserFromToken,
    request: &user_api::InviteUserRequest,
) -> UserResult<InviteMultipleUserResponse> {
    let inviter_user = user_from_token.get_user_from_db(state).await?;

    if inviter_user.get_email() == request.email {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User Inviting themselves".to_string(),
        )
        .into());
    }

    let role_info = roles::get_role_info_from_role_id(
        state,
        request.role_id.as_str(),
        user_from_token.merchant_id.as_str(),
        user_from_token.org_id.as_str(),
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    if !role_info.is_invitable() {
        return Err(UserErrors::InvalidRoleId.into())
            .attach_printable(format!("role_id = {} is not invitable", request.role_id));
    }

    let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
    let invitee_user = state
        .store
        .find_user_by_email(invitee_email.clone().get_secret().expose().as_str())
        .await;

    if let Ok(invitee_user) = invitee_user {
        handle_existing_user_invitation(state, user_from_token, request, invitee_user.into()).await
    } else if invitee_user
        .as_ref()
        .map_err(|e| e.current_context().is_db_not_found())
        .err()
        .unwrap_or(false)
    {
        handle_new_user_invitation(state, user_from_token, request).await
    } else {
        Err(UserErrors::InternalServerError.into())
    }
}

//TODO: send email
async fn handle_existing_user_invitation(
    state: &AppState,
    user_from_token: &auth::UserFromToken,
    request: &user_api::InviteUserRequest,
    invitee_user_from_db: domain::UserFromStorage,
) -> UserResult<InviteMultipleUserResponse> {
    let now = common_utils::date_time::now();
    state
        .store
        .insert_user_role(UserRoleNew {
            user_id: invitee_user_from_db.get_user_id().to_owned(),
            merchant_id: user_from_token.merchant_id.clone(),
            role_id: request.role_id.clone(),
            org_id: user_from_token.org_id.clone(),
            status: UserStatus::Active,
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

    Ok(InviteMultipleUserResponse {
        email: request.email.clone(),
        is_email_sent: false,
        password: None,
        error: None,
    })
}

async fn handle_new_user_invitation(
    state: &AppState,
    user_from_token: &auth::UserFromToken,
    request: &user_api::InviteUserRequest,
) -> UserResult<InviteMultipleUserResponse> {
    let new_user = domain::NewUser::try_from((request.clone(), user_from_token.clone()))?;

    new_user
        .insert_user_in_db(state.store.as_ref())
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
    #[cfg(feature = "email")]
    {
        let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
        let email_contents = email_types::InviteUser {
            recipient_email: invitee_email,
            user_name: domain::UserName::new(new_user.get_name())?,
            settings: state.conf.clone(),
            subject: "You have been invited to join Hyperswitch Community!",
            merchant_id: user_from_token.merchant_id.clone(),
        };
        let send_email_result = state
            .email_client
            .compose_and_send_email(
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
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
        dashboard_metadata::set_metadata(state.clone(), invited_user_token, set_metadata_request)
            .await?;
    }

    Ok(InviteMultipleUserResponse {
        is_email_sent,
        password: if cfg!(not(feature = "email")) {
            Some(new_user.get_password().get_secret())
        } else {
            None
        },
        email: request.email.clone(),
        error: None,
    })
}

#[cfg(feature = "email")]
pub async fn resend_invite(
    state: AppState,
    user_from_token: auth::UserFromToken,
    request: user_api::ReInviteUserRequest,
) -> UserResponse<()> {
    let invitee_email = domain::UserEmail::from_pii_email(request.email)?;
    let user: domain::UserFromStorage = state
        .store
        .find_user_by_email(invitee_email.clone().get_secret().expose().as_str())
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
                        "User role with user_id = {} and org_id = {} is not found",
                        user.get_user_id(),
                        user_from_token.merchant_id
                    ))
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    if !matches!(user_role.status, UserStatus::InvitationSent) {
        return Err(UserErrors::InvalidRoleOperation.into())
            .attach_printable("User status is not InvitationSent".to_string());
    }

    let email_contents = email_types::InviteUser {
        recipient_email: invitee_email,
        user_name: domain::UserName::new(user.get_name())?,
        settings: state.conf.clone(),
        subject: "You have been invited to join Hyperswitch Community!",
        merchant_id: user_from_token.merchant_id,
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
    if user_from_token.merchant_id == request.merchant_id {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User switching to same merchant id".to_string(),
        )
        .into());
    }

    let user = user_from_token.get_user_from_db(&state).await?;

    let role_info = roles::get_role_info_from_role_id(
        &state,
        &user_from_token.role_id,
        user_from_token.merchant_id.as_str(),
        user_from_token.org_id.as_str(),
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
            state,
            &user,
            request.merchant_id.clone(),
            org_id,
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
            .ok_or(UserErrors::InvalidRoleOperation.into())
            .attach_printable("User doesn't have access to switch")?;

        let token = utils::user::generate_jwt_auth_token(&state, &user, user_role).await?;
        (token, user_role.role_id.clone())
    };

    Ok(ApplicationResponse::Json(
        user_api::SwitchMerchantResponse {
            token,
            name: user.get_name(),
            email: user.get_email(),
            user_id: user.get_user_id().to_string(),
            verification_days_left: None,
            user_role: role_id,
            merchant_id: request.merchant_id,
        },
    ))
}

pub async fn create_merchant_account(
    state: AppState,
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

pub async fn list_merchant_ids_for_user(
    state: AppState,
    user_from_token: auth::UserFromToken,
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

    Ok(ApplicationResponse::Json(
        utils::user::get_multiple_merchant_details_with_status(user_roles, merchant_accounts)?,
    ))
}

pub async fn get_users_for_merchant_account(
    state: AppState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::GetUsersResponse> {
    let users_and_user_roles = state
        .store
        .find_users_and_roles_by_merchant_id(user_from_token.merchant_id.as_str())
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("No users for given merchant id")?;

    let users_user_roles_and_roles =
        futures::future::try_join_all(users_and_user_roles.into_iter().map(
            |(user, user_role)| async {
                let role_info = roles::get_role_info_from_role_id(
                    &state,
                    &user_role.role_id,
                    user_role.merchant_id.as_str(),
                    user_role.org_id.as_str(),
                )
                .await
                .to_not_found_response(UserErrors::InternalServerError)?;
                Ok::<_, error_stack::Report<UserErrors>>((user, user_role, role_info))
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
                role_id: user_role.role_id,
                role_name: role_info.get_role_name().to_string(),
                status: user_role.status.foreign_into(),
                last_modified_at: user_role.last_modified,
            }
        })
        .collect();

    Ok(ApplicationResponse::Json(user_api::GetUsersResponse(
        user_details_vec,
    )))
}

#[cfg(feature = "email")]
pub async fn verify_email_without_invite_checks(
    state: AppState,
    req: user_api::VerifyEmailRequest,
) -> UserResponse<user_api::DashboardEntryResponse> {
    let token = req.token.clone().expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;
    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;
    let user = state
        .store
        .find_user_by_email(email_token.get_email())
        .await
        .change_context(UserErrors::InternalServerError)?;
    let user = state
        .store
        .update_user_by_user_id(user.user_id.as_str(), storage_user::UserUpdate::VerifyUser)
        .await
        .change_context(UserErrors::InternalServerError)?;
    let user_from_db: domain::UserFromStorage = user.into();
    let user_role = user_from_db.get_role_from_db(state.clone()).await?;
    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|e| logger::error!(?e));
    let token = utils::user::generate_jwt_auth_token(&state, &user_from_db, &user_role).await?;

    Ok(ApplicationResponse::Json(
        utils::user::get_dashboard_entry_response(&state, user_from_db, user_role, token)?,
    ))
}

#[cfg(feature = "email")]
pub async fn verify_email(
    state: AppState,
    req: user_api::VerifyEmailRequest,
) -> UserResponse<user_api::SignInResponse> {
    let token = req.token.clone().expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user = state
        .store
        .find_user_by_email(email_token.get_email())
        .await
        .change_context(UserErrors::InternalServerError)?;

    let user = state
        .store
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

    Ok(ApplicationResponse::Json(
        signin_strategy.get_signin_response(&state).await?,
    ))
}

#[cfg(feature = "email")]
pub async fn send_verification_mail(
    state: AppState,
    req: user_api::SendVerifyEmailRequest,
) -> UserResponse<()> {
    let user_email = domain::UserEmail::try_from(req.email)?;
    let user = state
        .store
        .find_user_by_email(user_email.clone().get_secret().expose().as_str())
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
    state: AppState,
    req: auth::ReconUser,
) -> UserResponse<user_api::VerifyTokenResponse> {
    let user = state
        .store
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
    state: AppState,
    user_token: auth::UserFromToken,
    req: user_api::UpdateUserAccountDetailsRequest,
) -> UserResponse<()> {
    let user: domain::UserFromStorage = state
        .store
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
        password: None,
        is_verified: None,
        preferred_merchant_id: req.preferred_merchant_id,
    };

    state
        .store
        .update_user_by_user_id(user.get_user_id(), user_update)
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}
