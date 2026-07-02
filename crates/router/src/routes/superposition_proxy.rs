use actix_web::{web, HttpRequest, HttpResponse};
use external_services::superposition::{ContextPutRequest, ResolveConfigBody};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        superposition_proxy::{
            self, ListAuditLogsQuery, ListContextsQuery, ListDefaultConfigsQuery,
            ListDimensionsQuery, ResolveConfigExplanationRequest, ResolveDetailedConfigRequest,
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
    let request = ListContextsQuery::from(query.into_inner());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let request = request.clone();
            let org_id = org_id.clone();
            let workspace_id = workspace_id.clone();
            async move {
                superposition_proxy::handle_superposition_proxy_flow(state, user, request, org_id, workspace_id)
                    .await
            }
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
    query: web::Query<ListDefaultConfigsQuery>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListDefaultConfigs;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let request = query.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let request = request.clone();
            let org_id = org_id.clone();
            let workspace_id = workspace_id.clone();
            async move {
                superposition_proxy::handle_superposition_proxy_flow(state, user, request, org_id, workspace_id)
                    .await
            }
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
    query: web::Query<ListDimensionsQuery>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListDimensions;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let request = query.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let request = request.clone();
            let org_id = org_id.clone();
            let workspace_id = workspace_id.clone();
            async move {
                superposition_proxy::handle_superposition_proxy_flow(state, user, request, org_id, workspace_id)
                    .await
            }
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
    let request = body.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let request = request.clone();
            let org_id = org_id.clone();
            let workspace_id = workspace_id.clone();
            async move {
                superposition_proxy::handle_superposition_proxy_flow(state, user, request, org_id, workspace_id)
                    .await
            }
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
    let request = ResolveDetailedConfigRequest(body.into_inner().context);

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let request = request.clone();
            let org_id = org_id.clone();
            let workspace_id = workspace_id.clone();
            async move {
                superposition_proxy::handle_superposition_proxy_flow(state, user, request, org_id, workspace_id)
                    .await
            }
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

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionResolveConfigExplanation))]
pub async fn resolve_config_explanation(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<ResolveConfigBody>,
) -> HttpResponse {
    let flow = Flow::SuperpositionResolveConfigExplanation;
    let (org_id, workspace_id) = match superposition_proxy::extract_proxy_headers(&req) {
        Ok((org_id, workspace_id)) => (org_id, workspace_id),
        Err(response) => return response,
    };
    let request = ResolveConfigExplanationRequest {
        key: path.into_inner(),
        context: body.into_inner().context,
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let request = request.clone();
            let org_id = org_id.clone();
            let workspace_id = workspace_id.clone();
            async move {
                superposition_proxy::handle_superposition_proxy_flow(state, user, request, org_id, workspace_id)
                    .await
            }
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
    let request = ListAuditLogsQuery::from(query.into_inner());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let request = request.clone();
            let org_id = org_id.clone();
            let workspace_id = workspace_id.clone();
            async move {
                superposition_proxy::handle_superposition_proxy_flow(state, user, request, org_id, workspace_id)
                    .await
            }
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
