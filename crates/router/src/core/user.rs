use std::{
    collections::{HashMap, HashSet},
    ops::Not,
};

use api_models::{
    payments::RedirectionResponse,
    user::{self as user_api, InviteMultipleUserResponse, NameIdUnit},
};
use common_enums::{EntityType, UserAuthType};
use common_utils::{type_name, types::keymanager::Identifier};
#[cfg(feature = "email")]
use diesel_models::user_role::UserRoleUpdate;
use diesel_models::{
    enums::{TotpStatus, UserRoleVersion, UserStatus},
    organization::OrganizationBridge,
    user as storage_user,
    user_authentication_method::{UserAuthenticationMethodNew, UserAuthenticationMethodUpdate},
};
use error_stack::{report, ResultExt};
use masking::{ExposeInterface, PeekInterface, Secret};
#[cfg(feature = "email")]
use router_env::env;
use router_env::logger;
use storage_impl::errors::StorageError;
#[cfg(not(feature = "email"))]
use user_api::dashboard_metadata::SetMetaDataRequest;

#[cfg(feature = "v1")]
use super::admin;
use super::errors::{StorageErrorExt, UserErrors, UserResponse, UserResult};
#[cfg(feature = "email")]
use crate::services::email::types as email_types;
#[cfg(feature = "v1")]
use crate::types::transformers::ForeignFrom;
use crate::{
    consts,
    core::encryption::send_request_to_key_service_for_user,
    db::{
        domain::user_authentication_method::DEFAULT_USER_AUTH_METHOD,
        user_role::ListUserRolesByUserIdPayload,
    },
    routes::{app::ReqState, SessionState},
    services::{authentication as auth, authorization::roles, openidconnect, ApplicationResponse},
    types::{domain, transformers::ForeignInto},
    utils::{
        self,
        user::{theme as theme_utils, two_factor_auth as tfa_utils},
    },
};

pub mod dashboard_metadata;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;
pub mod theme;

#[cfg(feature = "email")]
pub async fn signup_with_merchant_id(
    state: SessionState,
    request: user_api::SignUpWithMerchantIdRequest,
    auth_id: Option<String>,
    theme_id: Option<String>,
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

    let _user_role = new_user
        .insert_org_level_user_role_in_db(
            state.clone(),
            common_utils::consts::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            UserStatus::Active,
        )
        .await?;

    let theme = theme_utils::get_theme_using_optional_theme_id(&state, theme_id).await?;

    let email_contents = email_types::ResetPassword {
        recipient_email: user_from_db.get_email().try_into()?,
        user_name: domain::UserName::new(user_from_db.get_name())?,
        settings: state.conf.clone(),
        subject: consts::user::EMAIL_SUBJECT_RESET_PASSWORD,
        auth_id,
        theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
        theme_config: theme
            .map(|theme| theme.email_config())
            .unwrap_or(state.conf.theme.email_config.clone()),
    };

    let send_email_result = state
        .email_client
        .compose_and_send_email(
            email_types::get_base_url(&state),
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await;

    logger::info!(?send_email_result);
    Ok(ApplicationResponse::Json(user_api::AuthorizeResponse {
        is_email_sent: send_email_result.is_ok(),
        user_id: user_from_db.get_user_id().to_string(),
    }))
}

pub async fn get_user_details(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::GetUserDetailsResponse> {
    let user = user_from_token.get_user_from_db(&state).await?;
    let verification_days_left = utils::user::get_verification_days_left(&state, &user)?;
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let theme = theme_utils::get_most_specific_theme_using_token_and_min_entity(
        &state,
        &user_from_token,
        EntityType::Profile,
    )
    .await?;

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
            profile_id: user_from_token.profile_id,
            entity_type: role_info.get_entity_type(),
            theme_id: theme.map(|theme| theme.theme_id),
        },
    ))
}

pub async fn signup_token_only_flow(
    state: SessionState,
    request: user_api::SignUpRequest,
) -> UserResponse<user_api::TokenResponse> {
    let user_email = domain::UserEmail::from_pii_email(request.email.clone())?;
    utils::user::validate_email_domain_auth_type_using_db(
        &state,
        &user_email,
        UserAuthType::Password,
    )
    .await?;

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
        .insert_org_level_user_role_in_db(
            state.clone(),
            common_utils::consts::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
            UserStatus::Active,
        )
        .await?;

    let next_flow =
        domain::NextFlow::from_origin(domain::Origin::SignUp, user_from_db.clone(), &state).await?;

    let token = next_flow
        .get_token_with_user_role(&state, &user_role)
        .await?;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    };
    auth::cookies::set_cookie_response(response, token)
}

pub async fn signin_token_only_flow(
    state: SessionState,
    request: user_api::SignInRequest,
) -> UserResponse<user_api::TokenResponse> {
    let user_email = domain::UserEmail::from_pii_email(request.email)?;

    utils::user::validate_email_domain_auth_type_using_db(
        &state,
        &user_email,
        UserAuthType::Password,
    )
    .await?;

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(&user_email)
        .await
        .to_not_found_response(UserErrors::InvalidCredentials)?
        .into();

    user_from_db.compare_password(&request.password)?;

    let next_flow =
        domain::NextFlow::from_origin(domain::Origin::SignIn, user_from_db.clone(), &state).await?;

    let token = next_flow.get_token(&state).await?;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    };
    auth::cookies::set_cookie_response(response, token)
}

#[cfg(feature = "email")]
pub async fn connect_account(
    state: SessionState,
    request: user_api::ConnectAccountRequest,
    auth_id: Option<String>,
    theme_id: Option<String>,
) -> UserResponse<user_api::ConnectAccountResponse> {
    let user_email = domain::UserEmail::from_pii_email(request.email.clone())?;

    utils::user::validate_email_domain_auth_type_using_db(
        &state,
        &user_email,
        UserAuthType::MagicLink,
    )
    .await?;

    let find_user = state.global_store.find_user_by_email(&user_email).await;

    if let Ok(found_user) = find_user {
        let user_from_db: domain::UserFromStorage = found_user.into();

        let theme = theme_utils::get_theme_using_optional_theme_id(&state, theme_id).await?;

        let email_contents = email_types::MagicLink {
            recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
            settings: state.conf.clone(),
            user_name: domain::UserName::new(user_from_db.get_name())?,
            subject: consts::user::EMAIL_SUBJECT_MAGIC_LINK,
            auth_id,
            theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
            theme_config: theme
                .map(|theme| theme.email_config())
                .unwrap_or(state.conf.theme.email_config.clone()),
        };

        let send_email_result = state
            .email_client
            .compose_and_send_email(
                email_types::get_base_url(&state),
                Box::new(email_contents),
                state.conf.proxy.https_url.as_ref(),
            )
            .await;
        logger::info!(?send_email_result);

        Ok(ApplicationResponse::Json(
            user_api::ConnectAccountResponse {
                is_email_sent: send_email_result.is_ok(),
                user_id: user_from_db.get_user_id().to_string(),
            },
        ))
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
        let _user_role = new_user
            .insert_org_level_user_role_in_db(
                state.clone(),
                common_utils::consts::ROLE_ID_ORGANIZATION_ADMIN.to_string(),
                UserStatus::Active,
            )
            .await?;

        let theme = theme_utils::get_theme_using_optional_theme_id(&state, theme_id).await?;

        let magic_link_email = email_types::VerifyEmail {
            recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
            settings: state.conf.clone(),
            subject: consts::user::EMAIL_SUBJECT_SIGNUP,
            auth_id,
            theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
            theme_config: theme
                .map(|theme| theme.email_config())
                .unwrap_or(state.conf.theme.email_config.clone()),
        };

        let magic_link_result = state
            .email_client
            .compose_and_send_email(
                email_types::get_base_url(&state),
                Box::new(magic_link_email),
                state.conf.proxy.https_url.as_ref(),
            )
            .await;

        logger::info!(?magic_link_result);

        if state.tenant.tenant_id.get_string_repr() == common_utils::consts::DEFAULT_TENANT {
            let welcome_to_community_email = email_types::WelcomeToCommunity {
                recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
                subject: consts::user::EMAIL_SUBJECT_WELCOME_TO_COMMUNITY,
            };

            let welcome_email_result = state
                .email_client
                .compose_and_send_email(
                    email_types::get_base_url(&state),
                    Box::new(welcome_to_community_email),
                    state.conf.proxy.https_url.as_ref(),
                )
                .await;

            logger::info!(?welcome_email_result);
        }

        return Ok(ApplicationResponse::Json(
            user_api::ConnectAccountResponse {
                is_email_sent: magic_link_result.is_ok(),
                user_id: user_from_db.get_user_id().to_string(),
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
    user_from_token: auth::UserIdFromAuth,
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
        .map_err(|error| logger::error!(?error));

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
            .map_err(|e| logger::error!("Error while deleting dashboard metadata {e:?}"))
            .ok();
    }

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "email")]
pub async fn forgot_password(
    state: SessionState,
    request: user_api::ForgotPasswordRequest,
    auth_id: Option<String>,
    theme_id: Option<String>,
) -> UserResponse<()> {
    let user_email = domain::UserEmail::from_pii_email(request.email)?;

    utils::user::validate_email_domain_auth_type_using_db(
        &state,
        &user_email,
        UserAuthType::Password,
    )
    .await?;

    let user_from_db = state
        .global_store
        .find_user_by_email(&user_email)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::UserNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })
        .map(domain::UserFromStorage::from)?;

    let theme = theme_utils::get_theme_using_optional_theme_id(&state, theme_id).await?;

    let email_contents = email_types::ResetPassword {
        recipient_email: domain::UserEmail::from_pii_email(user_from_db.get_email())?,
        settings: state.conf.clone(),
        user_name: domain::UserName::new(user_from_db.get_name())?,
        subject: consts::user::EMAIL_SUBJECT_RESET_PASSWORD,
        auth_id,
        theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
        theme_config: theme
            .map(|theme| theme.email_config())
            .unwrap_or(state.conf.theme.email_config.clone()),
    };

    state
        .email_client
        .compose_and_send_email(
            email_types::get_base_url(&state),
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
        .map_err(|error| logger::error!(?error));

    auth::cookies::remove_cookie_response()
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
        .find_user_by_email(&email_token.get_email()?)
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
            .map_err(|error| logger::error!(?error));
    }

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|error| logger::error!(?error));
    let _ = auth::blacklist::insert_user_in_blacklist(&state, &user.user_id)
        .await
        .map_err(|error| logger::error!(?error));

    auth::cookies::remove_cookie_response()
}

pub async fn invite_multiple_user(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    requests: Vec<user_api::InviteUserRequest>,
    req_state: ReqState,
    auth_id: Option<String>,
) -> UserResponse<Vec<InviteMultipleUserResponse>> {
    if requests.len() > 10 {
        return Err(report!(UserErrors::MaxInvitationsError))
            .attach_printable("Number of invite requests must not exceed 10");
    }

    let responses = futures::future::join_all(requests.into_iter().map(|request| async {
        match handle_invitation(&state, &user_from_token, &request, &req_state, &auth_id).await {
            Ok(response) => response,
            Err(error) => {
                logger::error!(invite_error=?error);

                InviteMultipleUserResponse {
                    email: request.email,
                    is_email_sent: false,
                    password: None,
                    error: Some(error.current_context().get_error_message().to_string()),
                }
            }
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
    auth_id: &Option<String>,
) -> UserResult<InviteMultipleUserResponse> {
    let inviter_user = user_from_token.get_user_from_db(state).await?;

    if inviter_user.get_email() == request.email {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User Inviting themselves".to_string(),
        )
        .into());
    }

    let role_info = roles::RoleInfo::from_role_id_in_lineage(
        state,
        &request.role_id,
        &user_from_token.merchant_id,
        &user_from_token.org_id,
        &user_from_token.profile_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .to_not_found_response(UserErrors::InvalidRoleId)?;

    if !role_info.is_invitable() {
        return Err(report!(UserErrors::InvalidRoleId))
            .attach_printable(format!("role_id = {} is not invitable", request.role_id));
    }

    let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
    let invitee_user = state.global_store.find_user_by_email(&invitee_email).await;

    if let Ok(invitee_user) = invitee_user {
        handle_existing_user_invitation(
            state,
            user_from_token,
            request,
            invitee_user.into(),
            role_info,
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
            role_info,
            req_state.clone(),
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
    role_info: roles::RoleInfo,
    auth_id: &Option<String>,
) -> UserResult<InviteMultipleUserResponse> {
    let now = common_utils::date_time::now();

    if state
        .global_store
        .find_user_role_by_user_id_and_lineage(
            invitee_user_from_db.get_user_id(),
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            &user_from_token.profile_id,
            UserRoleVersion::V1,
        )
        .await
        .is_err_and(|err| err.current_context().is_db_not_found())
        .not()
    {
        return Err(UserErrors::UserExists.into());
    }

    if state
        .global_store
        .find_user_role_by_user_id_and_lineage(
            invitee_user_from_db.get_user_id(),
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            &user_from_token.profile_id,
            UserRoleVersion::V2,
        )
        .await
        .is_err_and(|err| err.current_context().is_db_not_found())
        .not()
    {
        return Err(UserErrors::UserExists.into());
    }

    let (org_id, merchant_id, profile_id) = match role_info.get_entity_type() {
        EntityType::Tenant => {
            return Err(UserErrors::InvalidRoleOperationWithMessage(
                "Tenant roles are not allowed for this operation".to_string(),
            )
            .into());
        }
        EntityType::Organization => (Some(&user_from_token.org_id), None, None),
        EntityType::Merchant => (
            Some(&user_from_token.org_id),
            Some(&user_from_token.merchant_id),
            None,
        ),
        EntityType::Profile => (
            Some(&user_from_token.org_id),
            Some(&user_from_token.merchant_id),
            Some(&user_from_token.profile_id),
        ),
    };

    if state
        .global_store
        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
            user_id: invitee_user_from_db.get_user_id(),
            tenant_id: user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            org_id,
            merchant_id,
            profile_id,
            entity_id: None,
            version: None,
            status: None,
            limit: Some(1),
        })
        .await
        .is_ok_and(|data| data.is_empty().not())
    {
        return Err(UserErrors::UserExists.into());
    }

    let user_role = domain::NewUserRole {
        user_id: invitee_user_from_db.get_user_id().to_owned(),
        role_id: request.role_id.clone(),
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
        entity: domain::NoLevel,
    };

    let _user_role = match role_info.get_entity_type() {
        EntityType::Tenant => {
            return Err(UserErrors::InvalidRoleOperationWithMessage(
                "Tenant roles are not allowed for this operation".to_string(),
            )
            .into());
        }
        EntityType::Organization => {
            user_role
                .add_entity(domain::OrganizationLevel {
                    tenant_id: user_from_token
                        .tenant_id
                        .clone()
                        .unwrap_or(state.tenant.tenant_id.clone()),
                    org_id: user_from_token.org_id.clone(),
                })
                .insert_in_v2(state)
                .await?
        }
        EntityType::Merchant => {
            user_role
                .add_entity(domain::MerchantLevel {
                    tenant_id: user_from_token
                        .tenant_id
                        .clone()
                        .unwrap_or(state.tenant.tenant_id.clone()),
                    org_id: user_from_token.org_id.clone(),
                    merchant_id: user_from_token.merchant_id.clone(),
                })
                .insert_in_v2(state)
                .await?
        }
        EntityType::Profile => {
            user_role
                .add_entity(domain::ProfileLevel {
                    tenant_id: user_from_token
                        .tenant_id
                        .clone()
                        .unwrap_or(state.tenant.tenant_id.clone()),
                    org_id: user_from_token.org_id.clone(),
                    merchant_id: user_from_token.merchant_id.clone(),
                    profile_id: user_from_token.profile_id.clone(),
                })
                .insert_in_v2(state)
                .await?
        }
    };

    let is_email_sent;
    #[cfg(feature = "email")]
    {
        let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
        let entity = match role_info.get_entity_type() {
            EntityType::Tenant => {
                return Err(UserErrors::InvalidRoleOperationWithMessage(
                    "Tenant roles are not allowed for this operation".to_string(),
                )
                .into());
            }
            EntityType::Organization => email_types::Entity {
                entity_id: user_from_token.org_id.get_string_repr().to_owned(),
                entity_type: EntityType::Organization,
            },
            EntityType::Merchant => email_types::Entity {
                entity_id: user_from_token.merchant_id.get_string_repr().to_owned(),
                entity_type: EntityType::Merchant,
            },
            EntityType::Profile => email_types::Entity {
                entity_id: user_from_token.profile_id.get_string_repr().to_owned(),
                entity_type: EntityType::Profile,
            },
        };

        let theme = theme_utils::get_most_specific_theme_using_token_and_min_entity(
            state,
            user_from_token,
            role_info.get_entity_type(),
        )
        .await?;

        let email_contents = email_types::InviteUser {
            recipient_email: invitee_email,
            user_name: domain::UserName::new(invitee_user_from_db.get_name())?,
            settings: state.conf.clone(),
            subject: consts::user::EMAIL_SUBJECT_INVITATION,
            entity,
            auth_id: auth_id.clone(),
            theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
            theme_config: theme
                .map(|theme| theme.email_config())
                .unwrap_or(state.conf.theme.email_config.clone()),
        };

        is_email_sent = state
            .email_client
            .compose_and_send_email(
                email_types::get_base_url(state),
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
    role_info: roles::RoleInfo,
    req_state: ReqState,
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

    let user_role = domain::NewUserRole {
        user_id: new_user.get_user_id().to_owned(),
        role_id: request.role_id.clone(),
        status: invitation_status,
        created_by: user_from_token.user_id.clone(),
        last_modified_by: user_from_token.user_id.clone(),
        created_at: now,
        last_modified: now,
        entity: domain::NoLevel,
    };

    let _user_role = match role_info.get_entity_type() {
        EntityType::Tenant => {
            return Err(UserErrors::InvalidRoleOperationWithMessage(
                "Tenant roles are not allowed for this operation".to_string(),
            )
            .into());
        }
        EntityType::Organization => {
            user_role
                .add_entity(domain::OrganizationLevel {
                    tenant_id: user_from_token
                        .tenant_id
                        .clone()
                        .unwrap_or(state.tenant.tenant_id.clone()),
                    org_id: user_from_token.org_id.clone(),
                })
                .insert_in_v2(state)
                .await?
        }
        EntityType::Merchant => {
            user_role
                .add_entity(domain::MerchantLevel {
                    tenant_id: user_from_token
                        .tenant_id
                        .clone()
                        .unwrap_or(state.tenant.tenant_id.clone()),
                    org_id: user_from_token.org_id.clone(),
                    merchant_id: user_from_token.merchant_id.clone(),
                })
                .insert_in_v2(state)
                .await?
        }
        EntityType::Profile => {
            user_role
                .add_entity(domain::ProfileLevel {
                    tenant_id: user_from_token
                        .tenant_id
                        .clone()
                        .unwrap_or(state.tenant.tenant_id.clone()),
                    org_id: user_from_token.org_id.clone(),
                    merchant_id: user_from_token.merchant_id.clone(),
                    profile_id: user_from_token.profile_id.clone(),
                })
                .insert_in_v2(state)
                .await?
        }
    };

    let is_email_sent;

    #[cfg(feature = "email")]
    {
        // TODO: Adding this to avoid clippy lints
        // Will be adding actual usage for this variable later
        let _ = req_state.clone();
        let invitee_email = domain::UserEmail::from_pii_email(request.email.clone())?;
        let entity = match role_info.get_entity_type() {
            EntityType::Tenant => {
                return Err(UserErrors::InvalidRoleOperationWithMessage(
                    "Tenant roles are not allowed for this operation".to_string(),
                )
                .into());
            }
            EntityType::Organization => email_types::Entity {
                entity_id: user_from_token.org_id.get_string_repr().to_owned(),
                entity_type: EntityType::Organization,
            },
            EntityType::Merchant => email_types::Entity {
                entity_id: user_from_token.merchant_id.get_string_repr().to_owned(),
                entity_type: EntityType::Merchant,
            },
            EntityType::Profile => email_types::Entity {
                entity_id: user_from_token.profile_id.get_string_repr().to_owned(),
                entity_type: EntityType::Profile,
            },
        };

        let theme = theme_utils::get_most_specific_theme_using_token_and_min_entity(
            state,
            user_from_token,
            role_info.get_entity_type(),
        )
        .await?;

        let email_contents = email_types::InviteUser {
            recipient_email: invitee_email,
            user_name: domain::UserName::new(new_user.get_name())?,
            settings: state.conf.clone(),
            subject: consts::user::EMAIL_SUBJECT_INVITATION,
            entity,
            auth_id: auth_id.clone(),
            theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
            theme_config: theme
                .map(|theme| theme.email_config())
                .unwrap_or(state.conf.theme.email_config.clone()),
        };
        let send_email_result = state
            .email_client
            .compose_and_send_email(
                email_types::get_base_url(state),
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
            profile_id: user_from_token.profile_id.clone(),
            tenant_id: user_from_token.tenant_id.clone(),
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
        .find_user_by_email(&invitee_email)
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

    let user_role = match state
        .global_store
        .find_user_role_by_user_id_and_lineage(
            user.get_user_id(),
            user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            &user_from_token.org_id,
            &user_from_token.merchant_id,
            &user_from_token.profile_id,
            UserRoleVersion::V2,
        )
        .await
    {
        Ok(user_role) => Some(user_role),
        Err(err) => {
            if err.current_context().is_db_not_found() {
                None
            } else {
                return Err(report!(UserErrors::InternalServerError));
            }
        }
    };

    let user_role = match user_role {
        Some(user_role) => user_role,
        None => state
            .global_store
            .find_user_role_by_user_id_and_lineage(
                user.get_user_id(),
                user_from_token
                    .tenant_id
                    .as_ref()
                    .unwrap_or(&state.tenant.tenant_id),
                &user_from_token.org_id,
                &user_from_token.merchant_id,
                &user_from_token.profile_id,
                UserRoleVersion::V1,
            )
            .await
            .to_not_found_response(UserErrors::InvalidRoleOperationWithMessage(
                "User not found in records".to_string(),
            ))?,
    };

    if !matches!(user_role.status, UserStatus::InvitationSent) {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("User status is not InvitationSent".to_string());
    }

    let (entity_id, entity_type) = user_role
        .get_entity_id_and_type()
        .ok_or(UserErrors::InternalServerError)?;

    let invitee_role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_role.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let theme = theme_utils::get_most_specific_theme_using_token_and_min_entity(
        &state,
        &user_from_token,
        invitee_role_info.get_entity_type(),
    )
    .await?;

    let email_contents = email_types::InviteUser {
        recipient_email: invitee_email,
        user_name: domain::UserName::new(user.get_name())?,
        settings: state.conf.clone(),
        subject: consts::user::EMAIL_SUBJECT_INVITATION,
        entity: email_types::Entity {
            entity_id,
            entity_type,
        },
        auth_id: auth_id.clone(),
        theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
        theme_config: theme
            .map(|theme| theme.email_config())
            .unwrap_or(state.conf.theme.email_config.clone()),
    };

    state
        .email_client
        .compose_and_send_email(
            email_types::get_base_url(&state),
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "email")]
pub async fn accept_invite_from_email_token_only_flow(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    request: user_api::AcceptInviteFromEmailRequest,
) -> UserResponse<user_api::TokenResponse> {
    let token = request.token.expose();

    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(&email_token.get_email()?)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_user_id() != user_token.user_id {
        return Err(UserErrors::LinkInvalid.into());
    }

    let entity = email_token.get_entity().ok_or(UserErrors::LinkInvalid)?;

    let (org_id, merchant_id, profile_id) =
        utils::user_role::get_lineage_for_user_id_and_entity_for_accepting_invite(
            &state,
            &user_token.user_id,
            user_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            entity.entity_id.clone(),
            entity.entity_type,
        )
        .await
        .change_context(UserErrors::InternalServerError)?
        .ok_or(UserErrors::InternalServerError)?;

    let (update_v1_result, update_v2_result) = utils::user_role::update_v1_and_v2_user_roles_in_db(
        &state,
        user_from_db.get_user_id(),
        user_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
        &org_id,
        merchant_id.as_ref(),
        profile_id.as_ref(),
        UserRoleUpdate::UpdateStatus {
            status: UserStatus::Active,
            modified_by: user_from_db.get_user_id().to_owned(),
        },
    )
    .await;

    if update_v1_result
        .as_ref()
        .is_err_and(|err| !err.current_context().is_db_not_found())
        || update_v2_result
            .as_ref()
            .is_err_and(|err| !err.current_context().is_db_not_found())
    {
        return Err(report!(UserErrors::InternalServerError));
    }

    if update_v1_result.is_err() && update_v2_result.is_err() {
        return Err(report!(UserErrors::InvalidRoleOperation))
            .attach_printable("User not found in the organization")?;
    }

    if !user_from_db.is_verified() {
        let _ = state
            .global_store
            .update_user_by_user_id(
                user_from_db.get_user_id(),
                storage_user::UserUpdate::VerifyUser,
            )
            .await
            .map_err(|error| logger::error!(?error));
    }

    let _ = auth::blacklist::insert_email_token_in_blacklist(&state, &token)
        .await
        .map_err(|error| logger::error!(?error));

    let current_flow = domain::CurrentFlow::new(
        user_token,
        domain::SPTFlow::AcceptInvitationFromEmail.into(),
    )?;
    let next_flow = current_flow.next(user_from_db.clone(), &state).await?;

    let token = next_flow.get_token(&state).await?;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    };
    auth::cookies::set_cookie_response(response, token)
}

pub async fn create_internal_user(
    state: SessionState,
    request: user_api::CreateInternalUserRequest,
) -> UserResponse<()> {
    let key_manager_state = &(&state).into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &common_utils::id_type::MerchantId::get_internal_user_merchant_id(
                consts::user_role::INTERNAL_USER_MERCHANT_ID,
            ),
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

    let default_tenant_id = common_utils::id_type::TenantId::try_from_string(
        common_utils::consts::DEFAULT_TENANT.to_owned(),
    )
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Unable to parse default tenant id")?;

    if state.tenant.tenant_id != default_tenant_id {
        return Err(UserErrors::ForbiddenTenantId)
            .attach_printable("Operation allowed only for the default tenant");
    }

    let internal_merchant_id = common_utils::id_type::MerchantId::get_internal_user_merchant_id(
        consts::user_role::INTERNAL_USER_MERCHANT_ID,
    );

    let internal_merchant = state
        .store
        .find_merchant_account_by_merchant_id(key_manager_state, &internal_merchant_id, &key_store)
        .await
        .map_err(|e| {
            if e.current_context().is_db_not_found() {
                e.change_context(UserErrors::MerchantIdNotFound)
            } else {
                e.change_context(UserErrors::InternalServerError)
            }
        })?;

    let new_user = domain::NewUser::try_from((request, internal_merchant.organization_id.clone()))?;

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
        .get_no_level_user_role(
            common_utils::consts::ROLE_ID_INTERNAL_VIEW_ONLY_USER.to_string(),
            UserStatus::Active,
        )
        .add_entity(domain::MerchantLevel {
            tenant_id: default_tenant_id,
            org_id: internal_merchant.organization_id,
            merchant_id: internal_merchant_id,
        })
        .insert_in_v2(&state)
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn create_tenant_user(
    state: SessionState,
    request: user_api::CreateTenantUserRequest,
) -> UserResponse<()> {
    let key_manager_state = &(&state).into();

    let (merchant_id, org_id) = state
        .store
        .list_merchant_and_org_ids(key_manager_state, 1, None)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get merchants list for org")?
        .pop()
        .ok_or(UserErrors::InvalidRoleOperation)
        .attach_printable("No merchants found in the tenancy")?;

    let new_user = domain::NewUser::try_from((
        request,
        domain::MerchantAccountIdentifier {
            merchant_id,
            org_id,
        },
    ))?;
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
        .get_no_level_user_role(
            common_utils::consts::ROLE_ID_TENANT_ADMIN.to_string(),
            UserStatus::Active,
        )
        .add_entity(domain::TenantLevel {
            tenant_id: state.tenant.tenant_id.clone(),
        })
        .insert_in_v2(&state)
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
}

#[cfg(feature = "v1")]
pub async fn create_org_merchant_for_user(
    state: SessionState,
    req: user_api::UserOrgMerchantCreateRequest,
) -> UserResponse<()> {
    let db_organization = ForeignFrom::foreign_from(req.clone());
    let org: diesel_models::organization::Organization = state
        .accounts_store
        .insert_organization(db_organization)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let merchant_account_create_request =
        utils::user::create_merchant_account_request_for_org(req, org)?;

    admin::create_merchant_account(state.clone(), merchant_account_create_request)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error while creating a merchant")?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn create_merchant_account(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    req: user_api::UserMerchantCreate,
) -> UserResponse<()> {
    let user_from_db = user_from_token.get_user_from_db(&state).await?;

    let new_merchant = domain::NewUserMerchant::try_from((user_from_db, req, user_from_token))?;
    new_merchant
        .create_new_merchant_and_insert_in_db(state.to_owned())
        .await?;

    Ok(ApplicationResponse::StatusOk)
}

pub async fn list_user_roles_details(
    state: SessionState,
    user_from_token: auth::UserFromToken,
    request: user_api::GetUserRoleDetailsRequest,
    _req_state: ReqState,
) -> UserResponse<Vec<user_api::GetUserRoleDetailsResponseV2>> {
    let required_user = utils::user::get_user_from_db_by_email(&state, request.email.try_into()?)
        .await
        .to_not_found_response(UserErrors::InvalidRoleOperation)?;

    let requestor_role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .to_not_found_response(UserErrors::InternalServerError)
    .attach_printable("Failed to fetch role info")?;

    if requestor_role_info.is_internal() {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "Internal roles are not allowed for this operation".to_string(),
        )
        .into());
    }

    let user_roles_set = state
        .global_store
        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
            user_id: required_user.get_user_id(),
            tenant_id: user_from_token
                .tenant_id
                .as_ref()
                .unwrap_or(&state.tenant.tenant_id),
            org_id: Some(&user_from_token.org_id),
            merchant_id: (requestor_role_info.get_entity_type() <= EntityType::Merchant)
                .then_some(&user_from_token.merchant_id),
            profile_id: (requestor_role_info.get_entity_type() <= EntityType::Profile)
                .then_some(&user_from_token.profile_id),
            entity_id: None,
            version: None,
            status: None,
            limit: None,
        })
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to fetch user roles")?
        .into_iter()
        .collect::<HashSet<_>>();

    let org_name = state
        .accounts_store
        .find_organization_by_org_id(&user_from_token.org_id)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Org id not found")?
        .get_organization_name();

    let org = NameIdUnit {
        id: user_from_token.org_id.clone(),
        name: org_name,
    };

    let (merchant_ids, merchant_profile_ids) = user_roles_set.iter().try_fold(
        (Vec::new(), Vec::new()),
        |(mut merchant, mut merchant_profile), user_role| {
            let (_, entity_type) = user_role
                .get_entity_id_and_type()
                .ok_or(UserErrors::InternalServerError)
                .attach_printable("Failed to compute entity id and type")?;

            match entity_type {
                EntityType::Merchant => {
                    let merchant_id = user_role
                        .merchant_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)
                        .attach_printable(
                            "Merchant id not found in user role for merchant level entity",
                        )?;
                    merchant.push(merchant_id)
                }
                EntityType::Profile => {
                    let merchant_id = user_role
                        .merchant_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)
                        .attach_printable(
                            "Merchant id not found in user role for merchant level entity",
                        )?;
                    let profile_id = user_role
                        .profile_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)
                        .attach_printable(
                            "Profile id not found in user role for profile level entity",
                        )?;

                    merchant.push(merchant_id.clone());
                    merchant_profile.push((merchant_id, profile_id))
                }
                EntityType::Tenant | EntityType::Organization => (),
            };

            Ok::<_, error_stack::Report<UserErrors>>((merchant, merchant_profile))
        },
    )?;

    let merchant_map = state
        .store
        .list_multiple_merchant_accounts(&(&state).into(), merchant_ids)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Error while listing merchant accounts")?
        .into_iter()
        .map(|merchant_account| {
            (
                merchant_account.get_id().to_owned(),
                merchant_account.merchant_name.clone(),
            )
        })
        .collect::<HashMap<_, _>>();

    let key_manager_state = &(&state).into();

    let profile_map = futures::future::try_join_all(merchant_profile_ids.iter().map(
        |merchant_profile_id| async {
            let merchant_key_store = state
                .store
                .get_merchant_key_store_by_merchant_id(
                    key_manager_state,
                    &merchant_profile_id.0,
                    &state.store.get_master_key().to_vec().into(),
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to retrieve merchant key store by merchant_id")?;

            state
                .store
                .find_business_profile_by_profile_id(
                    key_manager_state,
                    &merchant_key_store,
                    &merchant_profile_id.1,
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to retrieve business profile")
        },
    ))
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to construct profile map")?
    .into_iter()
    .map(|profile| (profile.get_id().to_owned(), profile.profile_name))
    .collect::<HashMap<_, _>>();

    let role_name_map = futures::future::try_join_all(
        user_roles_set
            .iter()
            .map(|user_role| user_role.role_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|role_id| async {
                let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
                    &state,
                    &role_id,
                    &user_from_token.org_id,
                    user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                )
                .await
                .change_context(UserErrors::InternalServerError)?;

                Ok::<_, error_stack::Report<_>>((role_id, role_info.get_role_name().to_string()))
            }),
    )
    .await?
    .into_iter()
    .collect::<HashMap<_, _>>();

    let role_details_list: Vec<_> = user_roles_set
        .iter()
        .map(|user_role| {
            let (_, entity_type) = user_role
                .get_entity_id_and_type()
                .ok_or(UserErrors::InternalServerError)?;

            let (merchant, profile) = match entity_type {
                EntityType::Tenant | EntityType::Organization => (None, None),
                EntityType::Merchant => {
                    let merchant_id = &user_role
                        .merchant_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)?;

                    (
                        Some(NameIdUnit {
                            id: merchant_id.clone(),
                            name: merchant_map
                                .get(merchant_id)
                                .ok_or(UserErrors::InternalServerError)?
                                .to_owned(),
                        }),
                        None,
                    )
                }
                EntityType::Profile => {
                    let merchant_id = &user_role
                        .merchant_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)?;
                    let profile_id = &user_role
                        .profile_id
                        .clone()
                        .ok_or(UserErrors::InternalServerError)?;

                    (
                        Some(NameIdUnit {
                            id: merchant_id.clone(),
                            name: merchant_map
                                .get(merchant_id)
                                .ok_or(UserErrors::InternalServerError)?
                                .to_owned(),
                        }),
                        Some(NameIdUnit {
                            id: profile_id.clone(),
                            name: profile_map
                                .get(profile_id)
                                .ok_or(UserErrors::InternalServerError)?
                                .to_owned(),
                        }),
                    )
                }
            };

            Ok(user_api::GetUserRoleDetailsResponseV2 {
                role_id: user_role.role_id.clone(),
                org: org.clone(),
                merchant,
                profile,
                status: user_role.status.foreign_into(),
                entity_type,
                role_name: role_name_map
                    .get(&user_role.role_id)
                    .ok_or(UserErrors::InternalServerError)
                    .cloned()?,
            })
        })
        .collect::<Result<Vec<_>, UserErrors>>()?;

    Ok(ApplicationResponse::Json(role_details_list))
}

#[cfg(feature = "email")]
pub async fn verify_email_token_only_flow(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_api::VerifyEmailRequest,
) -> UserResponse<user_api::TokenResponse> {
    let token = req.token.clone().expose();
    let email_token = auth::decode_jwt::<email_types::EmailToken>(&token, &state)
        .await
        .change_context(UserErrors::LinkInvalid)?;

    auth::blacklist::check_email_token_in_blacklist(&state, &token).await?;

    let user_from_email = state
        .global_store
        .find_user_by_email(&email_token.get_email()?)
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
        .map_err(|error| logger::error!(?error));

    let current_flow = domain::CurrentFlow::new(user_token, domain::SPTFlow::VerifyEmail.into())?;
    let next_flow = current_flow.next(user_from_db, &state).await?;
    let token = next_flow.get_token(&state).await?;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: next_flow.get_flow().into(),
    };

    auth::cookies::set_cookie_response(response, token)
}

#[cfg(feature = "email")]
pub async fn send_verification_mail(
    state: SessionState,
    req: user_api::SendVerifyEmailRequest,
    auth_id: Option<String>,
    theme_id: Option<String>,
) -> UserResponse<()> {
    let user_email = domain::UserEmail::from_pii_email(req.email)?;

    utils::user::validate_email_domain_auth_type_using_db(
        &state,
        &user_email,
        UserAuthType::MagicLink,
    )
    .await?;

    let user = state
        .global_store
        .find_user_by_email(&user_email)
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

    let theme = theme_utils::get_theme_using_optional_theme_id(&state, theme_id).await?;

    let email_contents = email_types::VerifyEmail {
        recipient_email: domain::UserEmail::from_pii_email(user.email)?,
        settings: state.conf.clone(),
        subject: consts::user::EMAIL_SUBJECT_SIGNUP,
        auth_id,
        theme_id: theme.as_ref().map(|theme| theme.theme_id.clone()),
        theme_config: theme
            .map(|theme| theme.email_config())
            .unwrap_or(state.conf.theme.email_config.clone()),
    };

    state
        .email_client
        .compose_and_send_email(
            email_types::get_base_url(&state),
            Box::new(email_contents),
            state.conf.proxy.https_url.as_ref(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::StatusOk)
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

    let user_update = storage_user::UserUpdate::AccountUpdate {
        name: name.map(|name| name.get_secret().expose()),
        is_verified: None,
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
        .find_user_by_email(&email_token.get_email()?)
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

    let user_totp_attempts =
        tfa_utils::get_totp_attempts_from_redis(&state, &user_token.user_id).await?;

    if user_totp_attempts >= consts::user::TOTP_MAX_ATTEMPTS {
        return Err(UserErrors::MaxTotpAttemptsReached.into());
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
        let _ = tfa_utils::insert_totp_attempts_in_redis(
            &state,
            &user_token.user_id,
            user_totp_attempts + 1,
        )
        .await
        .inspect_err(|error| logger::error!(?error));
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
                    domain::types::crypto_operation::<String, masking::WithType>(
                        &(&state).into(),
                        type_name!(storage_user::User),
                        domain::types::CryptoOperation::Encrypt(totp.get_secret_base32().into()),
                        Identifier::User(key_store.user_id.clone()),
                        key_store.key.peek(),
                    )
                    .await
                    .and_then(|val| val.try_into_operation())
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
        .map_err(|error| logger::error!(?error));

    // This is not the main task of this API, so we don't throw error if this fails.
    // Any following API which requires TOTP will throw error if TOTP is not set in redis
    // and FE will ask user to enter TOTP again
    let _ = tfa_utils::insert_totp_in_redis(&state, &user_token.user_id)
        .await
        .map_err(|error| logger::error!(?error));

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

pub async fn transfer_user_key_store_keymanager(
    state: SessionState,
    req: user_api::UserKeyTransferRequest,
) -> UserResponse<user_api::UserTransferKeyResponse> {
    let db = &state.global_store;

    let key_stores = db
        .get_all_user_key_store(
            &(&state).into(),
            &state.store.get_master_key().to_vec().into(),
            req.from,
            req.limit,
        )
        .await
        .change_context(UserErrors::InternalServerError)?;

    Ok(ApplicationResponse::Json(
        user_api::UserTransferKeyResponse {
            total_transferred: send_request_to_key_service_for_user(&state, key_stores)
                .await
                .change_context(UserErrors::InternalServerError)?,
        },
    ))
}

pub async fn verify_recovery_code(
    state: SessionState,
    user_token: auth::UserIdFromAuth,
    req: user_api::VerifyRecoveryCodeRequest,
) -> UserResponse<()> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    if user_from_db.get_totp_status() != TotpStatus::Set {
        return Err(UserErrors::TwoFactorAuthNotSetup.into());
    }

    let user_recovery_code_attempts =
        tfa_utils::get_recovery_code_attempts_from_redis(&state, &user_token.user_id).await?;

    if user_recovery_code_attempts >= consts::user::RECOVERY_CODE_MAX_ATTEMPTS {
        return Err(UserErrors::MaxRecoveryCodeAttemptsReached.into());
    }

    let mut recovery_codes = user_from_db
        .get_recovery_codes()
        .ok_or(UserErrors::InternalServerError)?;

    let Some(matching_index) = utils::user::password::get_index_for_correct_recovery_code(
        &req.recovery_code,
        &recovery_codes,
    )?
    else {
        let _ = tfa_utils::insert_recovery_code_attempts_in_redis(
            &state,
            &user_token.user_id,
            user_recovery_code_attempts + 1,
        )
        .await
        .inspect_err(|error| logger::error!(?error));
        return Err(UserErrors::InvalidRecoveryCode.into());
    };

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

    if state.conf.user.force_two_factor_auth || !skip_two_factor_auth {
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

    let current_flow = domain::CurrentFlow::new(user_token.clone(), domain::SPTFlow::TOTP.into())?;
    let next_flow = current_flow.next(user_from_db, &state).await?;
    let token = next_flow.get_token(&state).await?;

    let _ = tfa_utils::delete_totp_attempts_from_redis(&state, &user_token.user_id)
        .await
        .inspect_err(|error| logger::error!(?error));
    let _ = tfa_utils::delete_recovery_code_attempts_from_redis(&state, &user_token.user_id)
        .await
        .inspect_err(|error| logger::error!(?error));

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

pub async fn check_two_factor_auth_status_with_attempts(
    state: SessionState,
    user_token: auth::UserIdFromAuth,
) -> UserResponse<user_api::TwoFactorStatus> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let is_skippable = state.conf.user.force_two_factor_auth.not();
    if user_from_db.get_totp_status() == TotpStatus::NotSet {
        return Ok(ApplicationResponse::Json(user_api::TwoFactorStatus {
            status: None,
            is_skippable,
        }));
    };

    let totp = user_api::TwoFactorAuthAttempts {
        is_completed: tfa_utils::check_totp_in_redis(&state, &user_token.user_id).await?,
        remaining_attempts: consts::user::TOTP_MAX_ATTEMPTS
            - tfa_utils::get_totp_attempts_from_redis(&state, &user_token.user_id).await?,
    };
    let recovery_code = user_api::TwoFactorAuthAttempts {
        is_completed: tfa_utils::check_recovery_code_in_redis(&state, &user_token.user_id).await?,
        remaining_attempts: consts::user::RECOVERY_CODE_MAX_ATTEMPTS
            - tfa_utils::get_recovery_code_attempts_from_redis(&state, &user_token.user_id).await?,
    };
    Ok(ApplicationResponse::Json(user_api::TwoFactorStatus {
        status: Some(user_api::TwoFactorAuthStatusResponseWithAttempts {
            totp,
            recovery_code,
        }),
        is_skippable,
    }))
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
    let id = uuid::Uuid::new_v4().to_string();
    let (private_config, public_config) = utils::user::construct_public_and_private_db_configs(
        &state,
        &req.auth_method,
        &user_auth_encryption_key,
        id.clone(),
    )
    .await?;

    let auth_methods = state
        .store
        .list_user_authentication_methods_for_owner_id(&req.owner_id)
        .await
        .change_context(UserErrors::InternalServerError)
        .attach_printable("Failed to get list of auth methods for the owner id")?;

    let (auth_id, email_domain) = if let Some(auth_method) = auth_methods.first() {
        let email_domain = match req.email_domain {
            Some(email_domain) => {
                if email_domain != auth_method.email_domain {
                    return Err(report!(UserErrors::InvalidAuthMethodOperationWithMessage(
                        "Email domain mismatch".to_string()
                    )));
                }

                email_domain
            }
            None => auth_method.email_domain.clone(),
        };

        (auth_method.auth_id.clone(), email_domain)
    } else {
        let email_domain =
            req.email_domain
                .ok_or(UserErrors::InvalidAuthMethodOperationWithMessage(
                    "Email domain not found".to_string(),
                ))?;

        (uuid::Uuid::new_v4().to_string(), email_domain)
    };

    for db_auth_method in auth_methods {
        let is_type_same = db_auth_method.auth_type == (&req.auth_method).foreign_into();
        let is_extra_identifier_same = match &req.auth_method {
            user_api::AuthConfig::OpenIdConnect { public_config, .. } => {
                let db_auth_name = db_auth_method
                    .public_config
                    .map(|config| {
                        utils::user::parse_value::<user_api::OpenIdConnectPublicConfig>(
                            config,
                            "OpenIdConnectPublicConfig",
                        )
                    })
                    .transpose()?
                    .map(|config| config.name);
                let req_auth_name = public_config.name;
                db_auth_name.is_some_and(|name| name == req_auth_name)
            }
            user_api::AuthConfig::Password | user_api::AuthConfig::MagicLink => true,
        };
        if is_type_same && is_extra_identifier_same {
            return Err(report!(UserErrors::UserAuthMethodAlreadyExists));
        }
    }

    let now = common_utils::date_time::now();
    state
        .store
        .insert_user_authentication_method(UserAuthenticationMethodNew {
            id,
            auth_id,
            owner_id: req.owner_id,
            owner_type: req.owner_type,
            auth_type: (&req.auth_method).foreign_into(),
            private_config,
            public_config,
            allow_signup: req.allow_signup,
            created_at: now,
            last_modified_at: now,
            email_domain,
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

    match req {
        user_api::UpdateUserAuthenticationMethodRequest::AuthMethod {
            id,
            auth_config: auth_method,
        } => {
            let (private_config, public_config) =
                utils::user::construct_public_and_private_db_configs(
                    &state,
                    &auth_method,
                    &user_auth_encryption_key,
                    id.clone(),
                )
                .await?;

            state
                .store
                .update_user_authentication_method(
                    &id,
                    UserAuthenticationMethodUpdate::UpdateConfig {
                        private_config,
                        public_config,
                    },
                )
                .await
                .map_err(|error| {
                    let user_error = match error.current_context() {
                        StorageError::ValueNotFound(_) => {
                            UserErrors::InvalidAuthMethodOperationWithMessage(
                                "Auth method not found".to_string(),
                            )
                        }
                        StorageError::DuplicateValue { .. } => {
                            UserErrors::UserAuthMethodAlreadyExists
                        }
                        _ => UserErrors::InternalServerError,
                    };
                    error.change_context(user_error)
                })?;
        }
        user_api::UpdateUserAuthenticationMethodRequest::EmailDomain {
            owner_id,
            email_domain,
        } => {
            let auth_methods = state
                .store
                .list_user_authentication_methods_for_owner_id(&owner_id)
                .await
                .change_context(UserErrors::InternalServerError)?;

            futures::future::try_join_all(auth_methods.iter().map(|auth_method| async {
                state
                    .store
                    .update_user_authentication_method(
                        &auth_method.id,
                        UserAuthenticationMethodUpdate::EmailDomain {
                            email_domain: email_domain.clone(),
                        },
                    )
                    .await
                    .to_duplicate_response(UserErrors::UserAuthMethodAlreadyExists)
            }))
            .await?;
        }
    }

    Ok(ApplicationResponse::StatusOk)
}

pub async fn list_user_authentication_methods(
    state: SessionState,
    req: user_api::GetUserAuthenticationMethodsRequest,
) -> UserResponse<Vec<user_api::UserAuthenticationMethodResponse>> {
    let user_authentication_methods = match (req.auth_id, req.email_domain) {
        (Some(auth_id), None) => state
            .store
            .list_user_authentication_methods_for_auth_id(&auth_id)
            .await
            .change_context(UserErrors::InternalServerError)?,
        (None, Some(email_domain)) => state
            .store
            .list_user_authentication_methods_for_email_domain(&email_domain)
            .await
            .change_context(UserErrors::InternalServerError)?,
        (Some(_), Some(_)) | (None, None) => {
            return Err(UserErrors::InvalidUserAuthMethodOperation.into());
        }
    };

    Ok(ApplicationResponse::Json(
        user_authentication_methods
            .into_iter()
            .map(|auth_method| {
                let auth_name = match (auth_method.auth_type, auth_method.public_config) {
                    (UserAuthType::OpenIdConnect, config) => {
                        let open_id_public_config: Option<user_api::OpenIdConnectPublicConfig> =
                            config
                                .map(|config| {
                                    utils::user::parse_value(config, "OpenIdConnectPublicConfig")
                                })
                                .transpose()?;
                        if let Some(public_config) = open_id_public_config {
                            Ok(Some(public_config.name))
                        } else {
                            Err(report!(UserErrors::InternalServerError))
                                .attach_printable("Public config not found for OIDC auth type")
                        }
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

#[cfg(feature = "v1")]
pub async fn get_sso_auth_url(
    state: SessionState,
    request: user_api::GetSsoAuthUrlRequest,
) -> UserResponse<()> {
    let user_authentication_method = state
        .store
        .get_user_authentication_method_by_id(request.id.as_str())
        .await
        .to_not_found_response(UserErrors::InvalidUserAuthMethodOperation)?;

    let open_id_private_config = utils::user::decrypt_oidc_private_config(
        &state,
        user_authentication_method.private_config,
        request.id.clone(),
    )
    .await?;

    let open_id_public_config = serde_json::from_value::<user_api::OpenIdConnectPublicConfig>(
        user_authentication_method
            .public_config
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("Public config not present")?,
    )
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Unable to parse OpenIdConnectPublicConfig")?;

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

    let open_id_private_config = utils::user::decrypt_oidc_private_config(
        &state,
        user_authentication_method.private_config,
        authentication_method_id,
    )
    .await?;

    let open_id_public_config = serde_json::from_value::<user_api::OpenIdConnectPublicConfig>(
        user_authentication_method
            .public_config
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("Public config not present")?,
    )
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Unable to parse OpenIdConnectPublicConfig")?;

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

    utils::user::validate_email_domain_auth_type_using_db(
        &state,
        &email,
        UserAuthType::OpenIdConnect,
    )
    .await?;

    // TODO: Use config to handle not found error
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_email(&email)
        .await
        .map(Into::into)
        .to_not_found_response(UserErrors::UserNotFound)?;

    if !user_from_db.is_verified() {
        state
            .global_store
            .update_user_by_user_id(
                user_from_db.get_user_id(),
                storage_user::UserUpdate::VerifyUser,
            )
            .await
            .change_context(UserErrors::InternalServerError)?;
    }

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

pub async fn terminate_auth_select(
    state: SessionState,
    user_token: auth::UserFromSinglePurposeToken,
    req: user_api::AuthSelectRequest,
) -> UserResponse<user_api::TokenResponse> {
    let user_from_db: domain::UserFromStorage = state
        .global_store
        .find_user_by_id(&user_token.user_id)
        .await
        .change_context(UserErrors::InternalServerError)?
        .into();

    let user_email = domain::UserEmail::from_pii_email(user_from_db.get_email())?;
    let auth_methods = state
        .store
        .list_user_authentication_methods_for_email_domain(user_email.extract_domain()?)
        .await
        .change_context(UserErrors::InternalServerError)?;

    let user_authentication_method = match (req.id, auth_methods.is_empty()) {
        (Some(id), _) => auth_methods
            .into_iter()
            .find(|auth_method| auth_method.id == id)
            .ok_or(UserErrors::InvalidUserAuthMethodOperation)?,
        (None, true) => DEFAULT_USER_AUTH_METHOD.clone(),
        (None, false) => return Err(UserErrors::InvalidUserAuthMethodOperation.into()),
    };

    let current_flow = domain::CurrentFlow::new(user_token, domain::SPTFlow::AuthSelect.into())?;
    let mut next_flow = current_flow.next(user_from_db.clone(), &state).await?;

    // Skip SSO if continue with password(TOTP)
    if next_flow.get_flow() == domain::UserFlow::SPTFlow(domain::SPTFlow::SSO)
        && !utils::user::is_sso_auth_type(user_authentication_method.auth_type)
    {
        next_flow = next_flow.skip(user_from_db, &state).await?;
    }
    let token = next_flow.get_token(&state).await?;

    auth::cookies::set_cookie_response(
        user_api::TokenResponse {
            token: token.clone(),
            token_type: next_flow.get_flow().into(),
        },
        token,
    )
}

pub async fn list_orgs_for_user(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<Vec<user_api::ListOrgsForUserResponse>> {
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    if role_info.is_internal() {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "Internal roles are not allowed for this operation".to_string(),
        )
        .into());
    }
    let orgs = match role_info.get_entity_type() {
        EntityType::Tenant => {
            let key_manager_state = &(&state).into();
            state
                .store
                .list_merchant_and_org_ids(
                    key_manager_state,
                    consts::user::ORG_LIST_LIMIT_FOR_TENANT,
                    None,
                )
                .await
                .change_context(UserErrors::InternalServerError)?
                .into_iter()
                .map(|(_, org_id)| org_id)
                .collect::<HashSet<_>>()
        }
        EntityType::Organization | EntityType::Merchant | EntityType::Profile => state
            .global_store
            .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                user_id: user_from_token.user_id.as_str(),
                tenant_id: user_from_token
                    .tenant_id
                    .as_ref()
                    .unwrap_or(&state.tenant.tenant_id),
                org_id: None,
                merchant_id: None,
                profile_id: None,
                entity_id: None,
                version: None,
                status: Some(UserStatus::Active),
                limit: None,
            })
            .await
            .change_context(UserErrors::InternalServerError)?
            .into_iter()
            .filter_map(|user_role| user_role.org_id)
            .collect::<HashSet<_>>(),
    };

    let resp = futures::future::try_join_all(
        orgs.iter()
            .map(|org_id| state.accounts_store.find_organization_by_org_id(org_id)),
    )
    .await
    .change_context(UserErrors::InternalServerError)?
    .into_iter()
    .map(|org| user_api::ListOrgsForUserResponse {
        org_id: org.get_organization_id(),
        org_name: org.get_organization_name(),
    })
    .collect::<Vec<_>>();

    if resp.is_empty() {
        Err(UserErrors::InternalServerError).attach_printable("No orgs found for a user")?;
    }

    Ok(ApplicationResponse::Json(resp))
}

pub async fn list_merchants_for_user_in_org(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<Vec<user_api::ListMerchantsForUserInOrgResponse>> {
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    if role_info.is_internal() {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "Internal roles are not allowed for this operation".to_string(),
        )
        .into());
    }

    let merchant_accounts = match role_info.get_entity_type() {
        EntityType::Tenant | EntityType::Organization => state
            .store
            .list_merchant_accounts_by_organization_id(&(&state).into(), &user_from_token.org_id)
            .await
            .change_context(UserErrors::InternalServerError)?,
        EntityType::Merchant | EntityType::Profile => {
            let merchant_ids = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id: user_from_token.user_id.as_str(),
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: Some(&user_from_token.org_id),
                    merchant_id: None,
                    profile_id: None,
                    entity_id: None,
                    version: None,
                    status: Some(UserStatus::Active),
                    limit: None,
                })
                .await
                .change_context(UserErrors::InternalServerError)?
                .into_iter()
                .filter_map(|user_role| user_role.merchant_id)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();

            state
                .store
                .list_multiple_merchant_accounts(&(&state).into(), merchant_ids)
                .await
                .change_context(UserErrors::InternalServerError)?
        }
    };

    if merchant_accounts.is_empty() {
        Err(UserErrors::InternalServerError).attach_printable("No merchant found for a user")?;
    }

    Ok(ApplicationResponse::Json(
        merchant_accounts
            .into_iter()
            .map(
                |merchant_account| user_api::ListMerchantsForUserInOrgResponse {
                    merchant_name: merchant_account.merchant_name.clone(),
                    merchant_id: merchant_account.get_id().to_owned(),
                },
            )
            .collect::<Vec<_>>(),
    ))
}

pub async fn list_profiles_for_user_in_org_and_merchant_account(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> UserResponse<Vec<user_api::ListProfilesForUserInOrgAndMerchantAccountResponse>> {
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)?;

    let key_manager_state = &(&state).into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            &user_from_token.merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(UserErrors::InternalServerError)?;
    let profiles = match role_info.get_entity_type() {
        EntityType::Tenant | EntityType::Organization | EntityType::Merchant => state
            .store
            .list_profile_by_merchant_id(
                key_manager_state,
                &key_store,
                &user_from_token.merchant_id,
            )
            .await
            .change_context(UserErrors::InternalServerError)?,
        EntityType::Profile => {
            let profile_ids = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id: user_from_token.user_id.as_str(),
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: Some(&user_from_token.org_id),
                    merchant_id: Some(&user_from_token.merchant_id),
                    profile_id: None,
                    entity_id: None,
                    version: None,
                    status: Some(UserStatus::Active),
                    limit: None,
                })
                .await
                .change_context(UserErrors::InternalServerError)?
                .into_iter()
                .filter_map(|user_role| user_role.profile_id)
                .collect::<HashSet<_>>();

            futures::future::try_join_all(profile_ids.iter().map(|profile_id| {
                state.store.find_business_profile_by_profile_id(
                    key_manager_state,
                    &key_store,
                    profile_id,
                )
            }))
            .await
            .change_context(UserErrors::InternalServerError)?
        }
    };

    if profiles.is_empty() {
        Err(UserErrors::InternalServerError).attach_printable("No profile found for a user")?;
    }

    Ok(ApplicationResponse::Json(
        profiles
            .into_iter()
            .map(
                |profile| user_api::ListProfilesForUserInOrgAndMerchantAccountResponse {
                    profile_id: profile.get_id().to_owned(),
                    profile_name: profile.profile_name,
                },
            )
            .collect::<Vec<_>>(),
    ))
}

pub async fn switch_org_for_user(
    state: SessionState,
    request: user_api::SwitchOrganizationRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::TokenResponse> {
    if user_from_token.org_id == request.org_id {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User switching to same org".to_string(),
        )
        .into());
    }

    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to retrieve role information")?;

    if role_info.is_internal() {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "Org switching not allowed for Internal role".to_string(),
        )
        .into());
    }

    let (merchant_id, profile_id, role_id) = match role_info.get_entity_type() {
        EntityType::Tenant => {
            let merchant_id = state
                .store
                .list_merchant_accounts_by_organization_id(&(&state).into(), &request.org_id)
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to get merchant list for org")?
                .pop()
                .ok_or(UserErrors::InvalidRoleOperation)
                .attach_printable("No merchants found for the org id")?
                .get_id()
                .to_owned();

            let key_store = state
                .store
                .get_merchant_key_store_by_merchant_id(
                    &(&state).into(),
                    &merchant_id,
                    &state.store.get_master_key().to_vec().into(),
                )
                .await
                .change_context(UserErrors::InternalServerError)?;

            let profile_id = state
                .store
                .list_profile_by_merchant_id(&(&state).into(), &key_store, &merchant_id)
                .await
                .change_context(UserErrors::InternalServerError)?
                .pop()
                .ok_or(UserErrors::InternalServerError)?
                .get_id()
                .to_owned();

            (merchant_id, profile_id, user_from_token.role_id)
        }
        EntityType::Organization | EntityType::Merchant | EntityType::Profile => {
            let user_role = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                    user_id: &user_from_token.user_id,
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: Some(&request.org_id),
                    merchant_id: None,
                    profile_id: None,
                    entity_id: None,
                    version: None,
                    status: Some(UserStatus::Active),
                    limit: Some(1),
                })
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to list user roles by user_id and org_id")?
                .pop()
                .ok_or(UserErrors::InvalidRoleOperationWithMessage(
                    "No user role found for the requested org_id".to_string(),
                ))?;

            let (merchant_id, profile_id) =
                utils::user_role::get_single_merchant_id_and_profile_id(&state, &user_role).await?;

            (merchant_id, profile_id, user_role.role_id)
        }
    };

    let token = utils::user::generate_jwt_auth_token_with_attributes(
        &state,
        user_from_token.user_id,
        merchant_id.clone(),
        request.org_id.clone(),
        role_id.clone(),
        profile_id.clone(),
        user_from_token.tenant_id.clone(),
    )
    .await?;

    utils::user_role::set_role_info_in_cache_by_role_id_org_id(
        &state,
        &role_id,
        &request.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: common_enums::TokenPurpose::UserInfo,
    };

    auth::cookies::set_cookie_response(response, token)
}

pub async fn switch_merchant_for_user_in_org(
    state: SessionState,
    request: user_api::SwitchMerchantRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::TokenResponse> {
    if user_from_token.merchant_id == request.merchant_id {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User switching to same merchant".to_string(),
        )
        .into());
    }

    let key_manager_state = &(&state).into();
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to retrieve role information")?;

    // Check if the role is internal and handle separately
    let (org_id, merchant_id, profile_id, role_id) = if role_info.is_internal() {
        let merchant_key_store = state
            .store
            .get_merchant_key_store_by_merchant_id(
                key_manager_state,
                &request.merchant_id,
                &state.store.get_master_key().to_vec().into(),
            )
            .await
            .to_not_found_response(UserErrors::MerchantIdNotFound)?;

        let merchant_account = state
            .store
            .find_merchant_account_by_merchant_id(
                key_manager_state,
                &request.merchant_id,
                &merchant_key_store,
            )
            .await
            .to_not_found_response(UserErrors::MerchantIdNotFound)?;

        let profile_id = state
            .store
            .list_profile_by_merchant_id(
                key_manager_state,
                &merchant_key_store,
                &request.merchant_id,
            )
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable("Failed to list business profiles by merchant_id")?
            .pop()
            .ok_or(UserErrors::InternalServerError)
            .attach_printable("No business profile found for the given merchant_id")?
            .get_id()
            .to_owned();

        (
            merchant_account.organization_id,
            request.merchant_id,
            profile_id,
            user_from_token.role_id.clone(),
        )
    } else {
        // Match based on the other entity types
        match role_info.get_entity_type() {
            EntityType::Tenant | EntityType::Organization => {
                let merchant_key_store = state
                    .store
                    .get_merchant_key_store_by_merchant_id(
                        key_manager_state,
                        &request.merchant_id,
                        &state.store.get_master_key().to_vec().into(),
                    )
                    .await
                    .to_not_found_response(UserErrors::MerchantIdNotFound)?;

                let merchant_id = state
                    .store
                    .find_merchant_account_by_merchant_id(
                        key_manager_state,
                        &request.merchant_id,
                        &merchant_key_store,
                    )
                    .await
                    .change_context(UserErrors::MerchantIdNotFound)?
                    .organization_id
                    .eq(&user_from_token.org_id)
                    .then(|| request.merchant_id.clone())
                    .ok_or_else(|| {
                        UserErrors::InvalidRoleOperationWithMessage(
                            "No such merchant_id found for the user in the org".to_string(),
                        )
                    })?;

                let profile_id = state
                    .store
                    .list_profile_by_merchant_id(
                        key_manager_state,
                        &merchant_key_store,
                        &merchant_id,
                    )
                    .await
                    .change_context(UserErrors::InternalServerError)
                    .attach_printable("Failed to list business profiles by merchant_id")?
                    .pop()
                    .ok_or(UserErrors::InternalServerError)
                    .attach_printable("No business profile found for the merchant_id")?
                    .get_id()
                    .to_owned();
                (
                    user_from_token.org_id.clone(),
                    merchant_id,
                    profile_id,
                    user_from_token.role_id.clone(),
                )
            }

            EntityType::Merchant | EntityType::Profile => {
                let user_role = state
                    .global_store
                    .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
                        user_id: &user_from_token.user_id,
                        tenant_id: user_from_token
                            .tenant_id
                            .as_ref()
                            .unwrap_or(&state.tenant.tenant_id),
                        org_id: Some(&user_from_token.org_id),
                        merchant_id: Some(&request.merchant_id),
                        profile_id: None,
                        entity_id: None,
                        version: None,
                        status: Some(UserStatus::Active),
                        limit: Some(1),
                    })
                    .await
                    .change_context(UserErrors::InternalServerError)
                    .attach_printable(
                        "Failed to list user roles for the given user_id, org_id and merchant_id",
                    )?
                    .pop()
                    .ok_or(UserErrors::InvalidRoleOperationWithMessage(
                        "No user role associated with the requested merchant_id".to_string(),
                    ))?;

                let (merchant_id, profile_id) =
                    utils::user_role::get_single_merchant_id_and_profile_id(&state, &user_role)
                        .await?;
                (
                    user_from_token.org_id,
                    merchant_id,
                    profile_id,
                    user_role.role_id,
                )
            }
        }
    };

    let token = utils::user::generate_jwt_auth_token_with_attributes(
        &state,
        user_from_token.user_id,
        merchant_id.clone(),
        org_id.clone(),
        role_id.clone(),
        profile_id,
        user_from_token.tenant_id.clone(),
    )
    .await?;

    utils::user_role::set_role_info_in_cache_by_role_id_org_id(
        &state,
        &role_id,
        &org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: common_enums::TokenPurpose::UserInfo,
    };

    auth::cookies::set_cookie_response(response, token)
}

pub async fn switch_profile_for_user_in_org_and_merchant(
    state: SessionState,
    request: user_api::SwitchProfileRequest,
    user_from_token: auth::UserFromToken,
) -> UserResponse<user_api::TokenResponse> {
    if user_from_token.profile_id == request.profile_id {
        return Err(UserErrors::InvalidRoleOperationWithMessage(
            "User switching to same profile".to_string(),
        )
        .into());
    }

    let key_manager_state = &(&state).into();
    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(UserErrors::InternalServerError)
    .attach_printable("Failed to retrieve role information")?;

    let (profile_id, role_id) = match role_info.get_entity_type() {
        EntityType::Tenant | EntityType::Organization | EntityType::Merchant => {
            let merchant_key_store = state
                .store
                .get_merchant_key_store_by_merchant_id(
                    key_manager_state,
                    &user_from_token.merchant_id,
                    &state.store.get_master_key().to_vec().into(),
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to retrieve merchant key store by merchant_id")?;

            let profile_id = state
                .store
                .find_business_profile_by_merchant_id_profile_id(
                    key_manager_state,
                    &merchant_key_store,
                    &user_from_token.merchant_id,
                    &request.profile_id,
                )
                .await
                .change_context(UserErrors::InvalidRoleOperationWithMessage(
                    "No such profile found for the merchant".to_string(),
                ))?
                .get_id()
                .to_owned();
            (profile_id, user_from_token.role_id)
        }

        EntityType::Profile => {
            let user_role = state
                .global_store
                .list_user_roles_by_user_id(ListUserRolesByUserIdPayload{
                    user_id:&user_from_token.user_id,
                    tenant_id: user_from_token
                        .tenant_id
                        .as_ref()
                        .unwrap_or(&state.tenant.tenant_id),
                    org_id: Some(&user_from_token.org_id),
                    merchant_id: Some(&user_from_token.merchant_id),
                    profile_id:Some(&request.profile_id),
                    entity_id: None,
                    version:None,
                    status: Some(UserStatus::Active),
                    limit: Some(1)
                }
                )
                .await
                .change_context(UserErrors::InternalServerError)
                .attach_printable("Failed to list user roles for the given user_id, org_id, merchant_id and profile_id")?
                .pop()
                .ok_or(UserErrors::InvalidRoleOperationWithMessage(
                    "No user role associated with the profile".to_string(),
                ))?;

            (request.profile_id, user_role.role_id)
        }
    };

    let token = utils::user::generate_jwt_auth_token_with_attributes(
        &state,
        user_from_token.user_id,
        user_from_token.merchant_id.clone(),
        user_from_token.org_id.clone(),
        role_id.clone(),
        profile_id,
        user_from_token.tenant_id.clone(),
    )
    .await?;

    utils::user_role::set_role_info_in_cache_by_role_id_org_id(
        &state,
        &role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await;

    let response = user_api::TokenResponse {
        token: token.clone(),
        token_type: common_enums::TokenPurpose::UserInfo,
    };

    auth::cookies::set_cookie_response(response, token)
}
