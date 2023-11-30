use actix_web::{web, HttpRequest, HttpResponse};
use api_models::{errors::types::ApiErrorResponse, user as user_api};
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
        |state, _, req_body| user_core::connect_account(state, req_body),
        &auth::NoAuth,
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
        |state, user, req| user_core::change_password(state, req, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn set_merchant_scoped_dashboard_metadata(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_api::dashboard_metadata::SetMetaDataRequest>,
) -> HttpResponse {
    let flow = Flow::SetDashboardMetadata;
    let mut payload = json_payload.into_inner();

    if let Err(e) = common_utils::errors::ReportSwitchExt::<(), ApiErrorResponse>::switch(
        set_ip_address_if_required(&mut payload, req.headers()),
    ) {
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
    let flow = Flow::GetMutltipleDashboardMetadata;
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
        |state, _, req| user_core::create_internal_user(state, req),
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
        |state, user, req| user_core::switch_merchant_id(state, req, user),
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
        |state, auth: auth::UserFromToken, json_payload| {
            user_core::create_merchant_account(state, auth, json_payload)
        },
        &auth::JWTAuth(Permission::MerchantAccountCreate),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
