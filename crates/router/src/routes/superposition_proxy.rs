use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, superposition_proxy},
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListContexts))]
pub async fn list_contexts(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    Box::pin(api::server_wrap(
        Flow::SuperpositionListContexts,
        state,
        &req,
        query.into_inner(),
        |state, user, params, _| async move {
            superposition_proxy::list_contexts(state, user, params).await
        },
        &auth::JWTAuth {
            permission: Permission::MerchantSuperpositionConfigRead,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListDefaultConfigs))]
pub async fn list_default_configs(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    Box::pin(api::server_wrap(
        Flow::SuperpositionListDefaultConfigs,
        state,
        &req,
        query.into_inner(),
        |state, user, params, _| async move {
            superposition_proxy::list_default_configs(state, user, params).await
        },
        &auth::JWTAuth {
            permission: Permission::MerchantSuperpositionConfigRead,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListDimensions))]
pub async fn list_dimensions(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    Box::pin(api::server_wrap(
        Flow::SuperpositionListDimensions,
        state,
        &req,
        query.into_inner(),
        |state, user, params, _| async move {
            superposition_proxy::list_dimensions(state, user, params).await
        },
        &auth::JWTAuth {
            permission: Permission::MerchantSuperpositionConfigRead,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionCreateContext))]
pub async fn create_context(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    Box::pin(api::server_wrap(
        Flow::SuperpositionCreateContext,
        state,
        &req,
        body.into_inner(),
        |state, user, body, _| async move {
            superposition_proxy::create_context(state, user, body).await
        },
        &auth::JWTAuth {
            permission: Permission::MerchantSuperpositionConfigWrite,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
