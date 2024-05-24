use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "dummy_connector")]
use api_models::user::sample_data::SampleDataRequest;
use api_models::{
    errors::types::ApiErrorResponse,
    user::{self as user_api},
};
use common_enums::TokenPurpose;
use common_utils::errors::ReportSwitchExt;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, user as user_core},
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
    utils::user::dashboard_metadata::{parse_string_to_enums, set_ip_address_if_required},
};

pub async fn get_user_details(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::GetUserDetails;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user, _, _| user_core::get_user_details(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
pub async fn user_signup_with_merchant_id(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignUpWithMerchantIdRequest>,
) -> HttpResponse {
    let flow = Flow::UserSignUpWithMerchantId;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body, _| user_core::signup_with_merchant_id(state, req_body),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn user_signup(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignUpRequest>,
    query: web::Query<user_api::TokenOnlyQueryParam>,
) -> HttpResponse {
    let flow = Flow::UserSignUp;
    let req_payload = json_payload.into_inner();
    let is_token_only = query.into_inner().token_only;
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body, _| async move {
            if let Some(true) = is_token_only {
                user_core::signup_token_only_flow(state, req_body).await
            } else {
                user_core::signup(state, req_body).await
            }
        },
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn user_signin(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignInRequest>,
    query: web::Query<user_api::TokenOnlyQueryParam>,
) -> HttpResponse {
    let flow = Flow::UserSignIn;
    let req_payload = json_payload.into_inner();
    let is_token_only = query.into_inner().token_only;
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body, _| async move {
            if let Some(true) = is_token_only {
                user_core::signin_token_only_flow(state, req_body).await
            } else {
                user_core::signin(state, req_body).await
            }
        },
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
pub async fn user_connect_account(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::ConnectAccountRequest>,
) -> HttpResponse {
    let flow = Flow::UserConnectAccount;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body, _| user_core::connect_account(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn signout(state: web::Data<AppState>, http_req: HttpRequest) -> HttpResponse {
    let flow = Flow::Signout;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, user, _, _| user_core::signout(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn change_password(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::ChangePasswordRequest>,
) -> HttpResponse {
    let flow = Flow::ChangePassword;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, user, req, _| user_core::change_password(state, req, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn set_dashboard_metadata(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::dashboard_metadata::SetMetaDataRequest>,
) -> HttpResponse {
    let flow = Flow::SetDashboardMetadata;
    let mut payload = json_payload.into_inner();

    if let Err(e) = ReportSwitchExt::<(), ApiErrorResponse>::switch(set_ip_address_if_required(
        &mut payload,
        req.headers(),
    )) {
        return api::log_and_return_error_response(e);
    }

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        user_core::dashboard_metadata::set_metadata,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_multiple_dashboard_metadata(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<user_api::dashboard_metadata::GetMultipleMetaDataRequest>,
) -> HttpResponse {
    let flow = Flow::GetMultipleDashboardMetadata;
    let payload = match ReportSwitchExt::<_, ApiErrorResponse>::switch(parse_string_to_enums(
        query.into_inner().keys,
    )) {
        Ok(payload) => payload,
        Err(e) => {
            return api::log_and_return_error_response(e);
        }
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        user_core::dashboard_metadata::get_multiple_metadata,
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn internal_user_signup(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::CreateInternalUserRequest>,
) -> HttpResponse {
    let flow = Flow::InternalUserSignup;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _, req, _| user_core::create_internal_user(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn switch_merchant_id(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SwitchMerchantIdRequest>,
) -> HttpResponse {
    let flow = Flow::SwitchMerchant;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, user, req, _| user_core::switch_merchant_id(state, req, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn user_merchant_account_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::UserMerchantCreate>,
) -> HttpResponse {
    let flow = Flow::UserMerchantAccountCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::UserFromToken, json_payload, _| {
            user_core::create_merchant_account(state, auth, json_payload)
        },
        &auth::JWTAuth(Permission::MerchantAccountCreate),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "dummy_connector")]
pub async fn generate_sample_data(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<SampleDataRequest>,
) -> impl actix_web::Responder {
    use crate::core::user::sample_data;

    let flow = Flow::GenerateSampleData;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        payload.into_inner(),
        sample_data::generate_sample_data_for_user,
        &auth::JWTAuth(Permission::PaymentWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(feature = "dummy_connector")]
pub async fn delete_sample_data(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<SampleDataRequest>,
) -> impl actix_web::Responder {
    use crate::core::user::sample_data;

    let flow = Flow::DeleteSampleData;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        payload.into_inner(),
        sample_data::delete_sample_data_for_user,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_merchants_for_user(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::UserMerchantAccountList;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user, _, _| user_core::list_merchants_for_user(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_merchants_for_user_with_spt(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::UserMerchantAccountList;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user, _, _| user_core::list_merchants_for_user(state, user),
        &auth::SinglePurposeJWTAuth(TokenPurpose::AcceptInvite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_user_role_details(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<user_api::GetUserRoleDetailsRequest>,
) -> HttpResponse {
    let flow = Flow::GetUserDetails;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        user_core::get_user_details_in_merchant_account,
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_users_for_merchant_account(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::ListUsersForMerchantAccount;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _, _| user_core::list_users_for_merchant_account(state, user),
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn rotate_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::RotatePasswordRequest>,
) -> HttpResponse {
    let flow = Flow::RotatePassword;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        user_core::rotate_password,
        &auth::SinglePurposeJWTAuth(TokenPurpose::ForceSetPassword),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
pub async fn forgot_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::ForgotPasswordRequest>,
) -> HttpResponse {
    let flow = Flow::ForgotPassword;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        |state, _, payload, _| user_core::forgot_password(state, payload),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
pub async fn reset_password(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::ResetPasswordRequest>,
    query: web::Query<user_api::TokenOnlyQueryParam>,
) -> HttpResponse {
    let flow = Flow::ResetPassword;
    let is_token_only = query.into_inner().token_only;
    if let Some(true) = is_token_only {
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            payload.into_inner(),
            |state, user, payload, _| {
                user_core::reset_password_token_only_flow(state, user, payload)
            },
            &auth::SinglePurposeJWTAuth(TokenPurpose::ResetPassword),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    } else {
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            payload.into_inner(),
            |state, _, payload, _| user_core::reset_password(state, payload),
            &auth::NoAuth,
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}
pub async fn invite_multiple_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<Vec<user_api::InviteUserRequest>>,
    query: web::Query<user_api::TokenOnlyQueryParam>,
) -> HttpResponse {
    let flow = Flow::InviteMultipleUser;
    let is_token_only = query.into_inner().token_only;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        |state, user, payload, req_state| {
            user_core::invite_multiple_user(state, user, payload, req_state, is_token_only)
        },
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
pub async fn resend_invite(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::ReInviteUserRequest>,
) -> HttpResponse {
    let flow = Flow::ReInviteUser;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        user_core::resend_invite,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
pub async fn accept_invite_from_email(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_api::AcceptInviteFromEmailRequest>,
    query: web::Query<user_api::TokenOnlyQueryParam>,
) -> HttpResponse {
    let flow = Flow::AcceptInviteFromEmail;
    let is_token_only = query.into_inner().token_only;
    if let Some(true) = is_token_only {
        Box::pin(api::server_wrap(
            flow.clone(),
            state,
            &req,
            payload.into_inner(),
            |state, user, req_payload, _| {
                user_core::accept_invite_from_email_token_only_flow(state, user, req_payload)
            },
            &auth::SinglePurposeJWTAuth(TokenPurpose::AcceptInvitationFromEmail),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    } else {
        Box::pin(api::server_wrap(
            flow,
            state.clone(),
            &req,
            payload.into_inner(),
            |state, _, request_payload, _| {
                user_core::accept_invite_from_email(state, request_payload)
            },
            &auth::NoAuth,
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}

#[cfg(feature = "email")]
pub async fn verify_email(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::VerifyEmailRequest>,
    query: web::Query<user_api::TokenOnlyQueryParam>,
) -> HttpResponse {
    let flow = Flow::VerifyEmail;
    let is_token_only = query.into_inner().token_only;
    if let Some(true) = is_token_only {
        Box::pin(api::server_wrap(
            flow.clone(),
            state,
            &http_req,
            json_payload.into_inner(),
            |state, user, req_payload, _| {
                user_core::verify_email_token_only_flow(state, user, req_payload)
            },
            &auth::SinglePurposeJWTAuth(TokenPurpose::VerifyEmail),
            api_locking::LockAction::NotApplicable,
        ))
        .await
    } else {
        Box::pin(api::server_wrap(
            flow.clone(),
            state,
            &http_req,
            json_payload.into_inner(),
            |state, _, req_payload, _| user_core::verify_email(state, req_payload),
            &auth::NoAuth,
            api_locking::LockAction::NotApplicable,
        ))
        .await
    }
}

#[cfg(feature = "email")]
pub async fn verify_email_request(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SendVerifyEmailRequest>,
) -> HttpResponse {
    let flow = Flow::VerifyEmailRequest;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _, req_body, _| user_core::send_verification_mail(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "recon")]
pub async fn verify_recon_token(state: web::Data<AppState>, http_req: HttpRequest) -> HttpResponse {
    let flow = Flow::ReconVerifyToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, user, _req, _| user_core::verify_token(state, user),
        &auth::ReconJWT,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn update_user_account_details(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::UpdateUserAccountDetailsRequest>,
) -> HttpResponse {
    let flow = Flow::UpdateUserAccountDetails;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        user_core::update_user_details,
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "email")]
pub async fn user_from_email(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::UserFromEmailRequest>,
) -> HttpResponse {
    let flow = Flow::UserFromEmail;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        |state, _: (), req_body, _| user_core::user_from_email(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn totp_begin(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::TotpBegin;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _, _| user_core::begin_totp(state, user),
        &auth::SinglePurposeJWTAuth(TokenPurpose::TOTP),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn totp_verify(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::VerifyTotpRequest>,
) -> HttpResponse {
    let flow = Flow::TotpVerify;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        |state, user, req_body, _| user_core::verify_totp(state, user, req_body),
        &auth::SinglePurposeJWTAuth(TokenPurpose::TOTP),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn verify_recovery_code(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::VerifyRecoveryCodeRequest>,
) -> HttpResponse {
    let flow = Flow::RecoveryCodeVerify;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        |state, user, req_body, _| user_core::verify_recovery_code(state, user, req_body),
        &auth::SinglePurposeJWTAuth(TokenPurpose::TOTP),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn generate_recovery_codes(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::RecoveryCodesGenerate;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _, _| user_core::generate_recovery_codes(state, user),
        &auth::SinglePurposeJWTAuth(TokenPurpose::TOTP),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn terminate_two_factor_auth(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<user_api::SkipTwoFactorAuthQueryParam>,
) -> HttpResponse {
    let flow = Flow::TerminateTwoFactorAuth;
    let skip_two_factor_auth = query.into_inner().skip_two_factor_auth.unwrap_or(false);

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _, _| user_core::terminate_two_factor_auth(state, user, skip_two_factor_auth),
        &auth::SinglePurposeJWTAuth(TokenPurpose::TOTP),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
