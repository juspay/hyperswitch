use actix_web::{web, HttpRequest, HttpResponse};
use api_models::user_role as user_role_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, user_role as user_role_core},
    services::{
        api,
        authentication::{self as auth, UserFromToken},
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

pub async fn get_role_from_token(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::GetRoleFromToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user: UserFromToken, _| user_role_core::role::get_role_from_token(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn create_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_role_api::role::CreateRoleRequest>,
) -> HttpResponse {
    let flow = Flow::CreateRole;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        user_role_core::role::create_role,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn update_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_role_api::role::UpdateRoleRequest>,
) -> HttpResponse {
    let flow = Flow::UpdateRole;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        user_role_core::role::update_role,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_all_roles(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::ListRoles;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _| user_role_core::role::list_invitable_roles(state, user),
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
    let request_payload = user_role_api::role::GetRoleRequest {
        role_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        request_payload,
        user_role_core::role::get_role,
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

pub async fn transfer_org_ownership(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_role_api::TransferOrgOwnershipRequest>,
) -> HttpResponse {
    let flow = Flow::TransferOrgOwnership;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        user_role_core::transfer_org_ownership,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn accept_invitation(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_role_api::AcceptInvitationRequest>,
) -> HttpResponse {
    let flow = Flow::AcceptInvitation;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        user_role_core::accept_invitation,
        &auth::UserWithoutMerchantJWTAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn delete_user_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<user_role_api::DeleteUserRoleRequest>,
) -> HttpResponse {
    let flow = Flow::DeleteUserRole;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload.into_inner(),
        user_role_core::delete_user_role,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
