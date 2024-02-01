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

/// Retrieves authorization information for a user based on the provided `AppState` and `HttpRequest`.
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

/// Asynchronously handles a request to list all roles. The method creates a Flow::ListRoles, wraps the flow in a server_wrap, and awaits the result. The server_wrap function takes in various parameters including the state, request, user_role_core::list_roles function, JWT authentication with UsersRead permission, and a LockAction. 
pub async fn list_all_roles(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
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

/// Asynchronously handles the HTTP request to retrieve a role based on the provided role ID. 
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

/// Asynchronously retrieves the role of a user from the provided token. 
/// 
/// # Arguments
/// * `state` - The web data state of the application.
/// * `req` - The HTTP request.
/// 
/// # Returns
/// The HTTP response containing the user's role.
pub async fn get_role_from_token(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::GetRoleFromToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user: UserFromToken, _| user_role_core::get_role_from_token(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Asynchronously updates the role of a user based on the provided JSON payload and HTTP request. 
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

/// Handles the HTTP request to accept an invitation by calling the `accept_invitation` function from the `user_role_core` module. It creates a new `AcceptInvitation` flow, extracts the JSON payload from the request, and passes it to the `accept_invitation` function after wrapping it using the `api::server_wrap` function with necessary parameters.
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

/// Asynchronously handles the request to delete a user role by wrapping the core logic in the API server. 
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
