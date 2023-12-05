use actix_web::{web, HttpRequest, HttpResponse};
use api_models::user_role as user_role_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, user_role as user_role_core},
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
};

pub async fn get_authorization_info(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::GetAuthorizationInfo;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, _: (), _| user_role_core::get_authorization_info(state),
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_roles(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::ListRoles;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, _: (), _| user_role_core::list_roles(state),
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::GetRole;
    let request_payload = user_role_api::GetRoleRequest {
        role_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        request_payload,
        |state, _: (), req| user_role_core::get_role(state, req),
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn update_user_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_role_api::UpdateUserRoleRequest>,
) -> HttpResponse {
    let flow = Flow::UpdateUserRole;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        user_role_core::update_user_role,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
