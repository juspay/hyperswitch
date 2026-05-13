use actix_web::{web, HttpRequest, HttpResponse};
use external_services::superposition::ContextPutRequest;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        superposition_proxy::{
            self as superposition_proxy, ProxyCreateContextRequest, ProxyListRequest,
        },
    },
    services::{api, authentication as auth, authorization::permissions::Permission},
};

fn extract_proxy_headers(req: &HttpRequest) -> Result<(String, String), HttpResponse> {
    let org_id = req
        .headers()
        .get("x-org-id")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": { "message": "missing required header: x-org-id" }
            }))
        })?;

    let workspace_id = req
        .headers()
        .get("x-workspace")
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| {
            HttpResponse::BadRequest().json(serde_json::json!({
                "error": { "message": "missing required header: x-workspace" }
            }))
        })?;

    Ok((org_id, workspace_id))
}

fn extract_proxy_list_request(
    req: &HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> Result<ProxyListRequest, HttpResponse> {
    let (org_id, workspace_id) = extract_proxy_headers(req)?;
    Ok(ProxyListRequest {
        params: query.into_inner(),
        org_id,
        workspace_id,
    })
}

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListContexts))]
pub async fn list_contexts(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListContexts;
    let payload = match extract_proxy_list_request(&req, query) {
        Ok(payload) => payload,
        Err(response) => return response,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, user, req, _| async move {
            superposition_proxy::list_contexts(state, user, req).await
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
    let flow = Flow::SuperpositionListDefaultConfigs;
    let payload = match extract_proxy_list_request(&req, query) {
        Ok(payload) => payload,
        Err(response) => return response,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, user, req, _| async move {
            superposition_proxy::list_default_configs(state, user, req).await
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
    let flow = Flow::SuperpositionListDimensions;
    let payload = match extract_proxy_list_request(&req, query) {
        Ok(payload) => payload,
        Err(response) => return response,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, user, req, _| async move {
            superposition_proxy::list_dimensions(state, user, req).await
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
    body: web::Json<ContextPutRequest>,
) -> HttpResponse {
    let flow = Flow::SuperpositionCreateContext;
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok((org_id, workspace_id)) => (org_id, workspace_id),
        Err(response) => return response,
    };

    let payload = ProxyCreateContextRequest {
        body: body.into_inner(),
        org_id,
        workspace_id,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, user, req, _| async move {
            superposition_proxy::create_context(state, user, req).await
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
