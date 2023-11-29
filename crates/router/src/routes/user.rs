use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(feature = "dummy_connector")]
use api_models::user::sample_data::SampleDataRequest;
use api_models::user::{self as user_api};
use router_env::Flow;

use super::AppState;
#[cfg(feature = "dummy_connector")]
use crate::services::authorization::permissions::Permission;
use crate::{
    core::{api_locking, user},
    services::{
        api,
        authentication::{self as auth},
    },
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
        |state, _, req_body| user::connect_account(state, req_body),
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
        |state, user, req| user::change_password(state, req, user),
        &auth::DashboardNoPermissionAuth,
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
    let flow = Flow::GenerateSampleData;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        payload.into_inner(),
        user::sample_data::generate_sample_data_for_user,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
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
    let flow = Flow::DeleteSampleData;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        payload.into_inner(),
        user::sample_data::delete_sample_data_for_user,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
