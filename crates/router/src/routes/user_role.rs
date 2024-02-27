use actix_web::{web, HttpRequest, HttpResponse};
use api_models::user_role::{self as user_role_api, role as role_api};
use router_env::Flow;

use super::AppState;
use crate::{
    core::{
        api_locking,
        user_role::{self as user_role_core, role as role_core},
    },
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
};

pub async fn get_authorization_info(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    query: web::Query<role_api::GetGroupsQueryParam>,
) -> HttpResponse {
    let flow = Flow::GetAuthorizationInfo;
    let respond_with_groups = query.into_inner().groups.unwrap_or(false);
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, _: (), _| async move {
            // TODO: Permissions to be deprecated once groups are stable
            if respond_with_groups {
                user_role_core::get_authorization_info_with_groups(state).await
            } else {
                user_role_core::get_authorization_info_with_modules(state).await
            }
        },
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
        |state, user, _| role_core::get_role_from_token(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn create_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<role_api::CreateRoleRequest>,
) -> HttpResponse {
    let flow = Flow::CreateRole;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        role_core::create_role,
        &auth::JWTAuth(Permission::UsersWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_all_roles(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<role_api::GetGroupsQueryParam>,
) -> HttpResponse {
    let flow = Flow::ListRoles;
    let respond_with_groups = query.into_inner().groups.unwrap_or(false);
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _| async move {
            // TODO: Permissions to be deprecated once groups are stable
            if respond_with_groups {
                role_core::list_invitable_roles_with_groups(state, user).await
            } else {
                role_core::list_invitable_roles_with_permissions(state, user).await
            }
        },
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<role_api::GetGroupsQueryParam>,
) -> HttpResponse {
    let flow = Flow::GetRole;
    let request_payload = user_role_api::role::GetRoleRequest {
        role_id: path.into_inner(),
    };
    let respond_with_groups = query.into_inner().groups.unwrap_or(false);
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        request_payload,
        |state, user, payload| async move {
            // TODO: Permissions to be deprecated once groups are stable
            if respond_with_groups {
                role_core::get_role_with_groups(state, user, payload).await
            } else {
                role_core::get_role_with_permissions(state, user, payload).await
            }
        },
        &auth::JWTAuth(Permission::UsersRead),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn update_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<role_api::UpdateRoleRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::UpdateRole;
    let role_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        |state, user, req| role_core::update_role(state, user, req, &role_id),
        &auth::JWTAuth(Permission::UsersWrite),
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
