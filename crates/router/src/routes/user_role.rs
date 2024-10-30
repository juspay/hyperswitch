use actix_web::{web, HttpRequest, HttpResponse};
use api_models::user_role::{self as user_role_api, role as role_api};
use common_enums::TokenPurpose;
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

// TODO: To be deprecated
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
        |state, _: (), _, _| async move {
            user_role_core::get_authorization_info_with_groups(state).await
        },
        &auth::JWTAuth {
            permission: Permission::MerchantUserRead,
        },
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
        |state, user, _, _| async move {
            role_core::get_role_from_token_with_groups(state, user).await
        },
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_groups_and_resources_for_role_from_token(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::GetRoleFromTokenV2;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user, _, _| async move {
            role_core::get_groups_and_resources_for_role_from_token(state, user).await
        },
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
        &auth::JWTAuth {
            permission: Permission::MerchantUserWrite,
        },
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
        |state, user, payload, _| async move {
            role_core::get_role_with_groups(state, user, payload).await
        },
        &auth::JWTAuth {
            permission: Permission::ProfileUserRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_parent_info_for_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::GetRoleV2;
    let request_payload = user_role_api::role::GetRoleRequest {
        role_id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        request_payload,
        |state, user, payload, _| async move {
            role_core::get_parent_info_for_role(state, user, payload).await
        },
        &auth::JWTAuth {
            permission: Permission::ProfileUserRead,
        },
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
        |state, user, req, _| role_core::update_role(state, user, req, &role_id),
        &auth::JWTAuth {
            permission: Permission::MerchantUserWrite,
        },
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
        &auth::JWTAuth {
            permission: Permission::ProfileUserWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn accept_invitations_v2(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_role_api::AcceptInvitationsV2Request>,
) -> HttpResponse {
    let flow = Flow::AcceptInvitationsV2;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        |state, user, req_body, _| user_role_core::accept_invitations_v2(state, user, req_body),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn accept_invitations_pre_auth(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<user_role_api::AcceptInvitationsPreAuthRequest>,
) -> HttpResponse {
    let flow = Flow::AcceptInvitationsPreAuth;
    let payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        payload,
        |state, user, req_body, _| async move {
            user_role_core::accept_invitations_pre_auth(state, user, req_body).await
        },
        &auth::SinglePurposeJWTAuth(TokenPurpose::AcceptInvite),
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
        &auth::JWTAuth {
            permission: Permission::ProfileUserWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_role_information(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::GetRolesInfo;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |_, _: (), _, _| async move {
            user_role_core::get_authorization_info_with_group_tag().await
        },
        &auth::JWTAuth {
            permission: Permission::ProfileUserRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_parent_group_info(
    state: web::Data<AppState>,
    http_req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::GetParentGroupInfo;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, user_from_token, _, _| async move {
            user_role_core::get_parent_group_info(state, user_from_token).await
        },
        &auth::JWTAuth {
            permission: Permission::ProfileUserRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_users_in_lineage(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<user_role_api::ListUsersInEntityRequest>,
) -> HttpResponse {
    let flow = Flow::ListUsersInLineage;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        query.into_inner(),
        |state, user_from_token, request, _| {
            user_role_core::list_users_in_lineage(state, user_from_token, request)
        },
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_roles_with_info(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<role_api::ListRolesRequest>,
) -> HttpResponse {
    let flow = Flow::ListRolesV2;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        query.into_inner(),
        |state, user_from_token, request, _| {
            role_core::list_roles_with_info(state, user_from_token, request)
        },
        &auth::JWTAuth {
            permission: Permission::ProfileUserRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_invitable_roles_at_entity_level(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<role_api::ListRolesAtEntityLevelRequest>,
) -> HttpResponse {
    let flow = Flow::ListInvitableRolesAtEntityLevel;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        query.into_inner(),
        |state, user_from_token, req, _| {
            role_core::list_roles_at_entity_level(
                state,
                user_from_token,
                req,
                role_api::RoleCheckType::Invite,
            )
        },
        &auth::JWTAuth {
            permission: Permission::ProfileUserRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_updatable_roles_at_entity_level(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<role_api::ListRolesAtEntityLevelRequest>,
) -> HttpResponse {
    let flow = Flow::ListUpdatableRolesAtEntityLevel;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        query.into_inner(),
        |state, user_from_token, req, _| {
            role_core::list_roles_at_entity_level(
                state,
                user_from_token,
                req,
                role_api::RoleCheckType::Update,
            )
        },
        &auth::JWTAuth {
            permission: Permission::ProfileUserRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_invitations_for_user(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::ListInvitationsForUser;

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, user_id_from_token, _, _| {
            user_role_core::list_invitations_for_user(state, user_id_from_token)
        },
        &auth::SinglePurposeOrLoginTokenAuth(TokenPurpose::AcceptInvite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
