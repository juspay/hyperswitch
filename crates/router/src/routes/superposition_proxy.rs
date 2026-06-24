use actix_web::{web, HttpRequest, HttpResponse};
use external_services::superposition::{
    context_put_from_request, ContextPutRequest, CreateContextInputBuilder, ResolveConfigBody,
};
use router_env::{instrument, logger, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        superposition_proxy::{
            self, ListAuditLogsQuery, ListContextsQuery, SuperpositionListQuery,
        },
    },
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListContexts))]
pub async fn list_contexts(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListContexts;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let params = ListContextsQuery::from(query.into_inner());
    let input = params.into_input(org_id, workspace_id);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let input = input.clone();
            async move { superposition_proxy::list_contexts(state, user, input).await }
        },
        &auth::JWTAuth {
            permission: Permission::ProfileSuperpositionConfigRead,
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
    query: web::Query<SuperpositionListQuery>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListDefaultConfigs;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let input = query
        .into_inner()
        .into_default_configs_input(org_id, workspace_id);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let input = input.clone();
            async move { superposition_proxy::list_default_configs(state, user, input).await }
        },
        &auth::JWTAuth {
            permission: Permission::ProfileSuperpositionConfigRead,
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
    query: web::Query<SuperpositionListQuery>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListDimensions;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let input = query
        .into_inner()
        .into_dimensions_input(org_id, workspace_id);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let input = input.clone();
            async move { superposition_proxy::list_dimensions(state, user, input).await }
        },
        &auth::JWTAuth {
            permission: Permission::ProfileSuperpositionConfigRead,
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
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok((org_id, workspace_id)) => (org_id, workspace_id),
        Err(response) => return response,
    };

    let context_put = match context_put_from_request(&body.into_inner()) {
        Ok(context_put) => context_put,
        Err(error) => {
            logger::error!(
                ?error,
                "superposition create_context failed to build ContextPut"
            );
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": { "message": "invalid context request body" }
            }));
        }
    };

    let input = CreateContextInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id)
        .request(context_put);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let input = input.clone();
            async move { superposition_proxy::create_context(state, user, input).await }
        },
        &auth::JWTAuth {
            permission: Permission::ProfileSuperpositionConfigWrite,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionResolveDetailedConfig))]
pub async fn resolve_detailed_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ResolveConfigBody>,
) -> HttpResponse {
    let flow = Flow::SuperpositionResolveDetailedConfig;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok((org_id, workspace_id)) => (org_id, workspace_id),
        Err(response) => return response,
    };

    let input = superposition_proxy::build_resolve_detailed_config_input(
        org_id,
        workspace_id,
        body.into_inner(),
    );

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let input = input.clone();
            async move { superposition_proxy::resolve_detailed_config(state, user, input).await }
        },
        &auth::JWTAuth {
            permission: Permission::ProfileSuperpositionConfigRead,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListAuditLogs))]
pub async fn list_audit_logs(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListAuditLogs;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let params = ListAuditLogsQuery::from(query.into_inner());
    let input = params.into_input(org_id, workspace_id);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let input = input.clone();
            async move { superposition_proxy::list_audit_logs(state, user, input).await }
        },
        &auth::JWTAuth {
            permission: Permission::ProfileSuperpositionConfigRead,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
