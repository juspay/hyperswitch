use std::collections::HashMap;

use actix_web::{web, HttpRequest, HttpResponse};
use external_services::superposition::ContextPutRequest;
use hyperswitch_masking::Secret;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        superposition_proxy::{
            self as superposition_proxy, ListAuditLogsRequest, ListContextsRequest,
            ListDefaultConfigsRequest, ListDimensionsRequest, ProxyCreateContextRequest,
            ProxyResolveConfigRequest, ResolveConfigBody,
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

fn parse_list_contexts_request(
    org_id: String,
    workspace_id: String,
    params: Vec<(String, String)>,
) -> ListContextsRequest {
    let mut count = None;
    let mut page = None;
    let mut all = None;
    let mut prefix = Vec::new();
    let mut sort_on = None;
    let mut sort_by = None;
    let mut created_by = Vec::new();
    let mut last_modified_by = Vec::new();
    let mut plaintext = None;
    let mut dimension_params = HashMap::new();

    for (key, value) in params {
        match key.as_str() {
            k if k.starts_with("dimension[") => {
                dimension_params.insert(key, value);
            }
            "count" => count = value.parse().ok(),
            "page" => page = value.parse().ok(),
            "all" => all = value.parse().ok(),
            "prefix" => prefix.push(value),
            "sort_on" => sort_on = Some(value),
            "sort_by" => sort_by = Some(value),
            "created_by" => created_by.push(value),
            "last_modified_by" => last_modified_by.push(value),
            "plaintext" => plaintext = Some(value),
            _ => {}
        }
    }

    ListContextsRequest {
        org_id: Secret::new(org_id),
        workspace_id: Secret::new(workspace_id),
        count,
        page,
        all,
        prefix: (!prefix.is_empty()).then_some(prefix),
        sort_on,
        sort_by,
        created_by: (!created_by.is_empty()).then_some(created_by),
        last_modified_by: (!last_modified_by.is_empty()).then_some(last_modified_by),
        plaintext,
        dimension_params,
    }
}

fn parse_list_default_configs_request(
    org_id: String,
    workspace_id: String,
    params: Vec<(String, String)>,
) -> ListDefaultConfigsRequest {
    let mut count = None;
    let mut page = None;
    let mut all = None;
    let mut name = None;

    for (key, value) in params {
        match key.as_str() {
            "count" => count = value.parse().ok(),
            "page" => page = value.parse().ok(),
            "all" => all = value.parse().ok(),
            "name" => name = Some(value),
            _ => {}
        }
    }

    ListDefaultConfigsRequest {
        org_id: Secret::new(org_id),
        workspace_id: Secret::new(workspace_id),
        count,
        page,
        all,
        name,
    }
}

fn parse_list_dimensions_request(
    org_id: String,
    workspace_id: String,
    params: Vec<(String, String)>,
) -> ListDimensionsRequest {
    let mut count = None;
    let mut page = None;
    let mut all = None;

    for (key, value) in params {
        match key.as_str() {
            "count" => count = value.parse().ok(),
            "page" => page = value.parse().ok(),
            "all" => all = value.parse().ok(),
            _ => {}
        }
    }

    ListDimensionsRequest {
        org_id: Secret::new(org_id),
        workspace_id: Secret::new(workspace_id),
        count,
        page,
        all,
    }
}

fn parse_list_audit_logs_request(
    org_id: String,
    workspace_id: String,
    params: Vec<(String, String)>,
) -> ListAuditLogsRequest {
    let mut count = None;
    let mut page = None;
    let mut all = None;
    let mut from_date = None;
    let mut to_date = None;
    let mut table: Vec<String> = Vec::new();
    let mut action: Vec<String> = Vec::new();
    let mut username = None;
    let mut sort_by = None;
    let mut dimension_params = HashMap::new();

    for (key, value) in params {
        match key.as_str() {
            k if k.starts_with("dimension[") => {
                dimension_params.insert(key, value);
            }
            "count" => count = value.parse().ok(),
            "page" => page = value.parse().ok(),
            "all" => all = value.parse().ok(),
            "from_date" => from_date = Some(value),
            "to_date" => to_date = Some(value),
            "table" => table.extend(value.split(',').map(|s| s.trim().to_owned())),
            "action" => action.extend(value.split(',').map(|s| s.trim().to_owned())),
            "username" => username = Some(value),
            "sort_by" => sort_by = Some(value),
            _ => {}
        }
    }

    ListAuditLogsRequest {
        org_id: Secret::new(org_id),
        workspace_id: Secret::new(workspace_id),
        count,
        page,
        all,
        from_date,
        to_date,
        table: (!table.is_empty()).then_some(table),
        action: (!action.is_empty()).then_some(action),
        username,
        sort_by,
        dimension_params,
    }
}

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListContexts))]
pub async fn list_contexts(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListContexts;
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let payload = parse_list_contexts_request(org_id, workspace_id, query.into_inner());

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
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let payload = parse_list_default_configs_request(org_id, workspace_id, query.into_inner());

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
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let payload = parse_list_dimensions_request(org_id, workspace_id, query.into_inner());

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
        org_id: Secret::new(org_id),
        workspace_id: Secret::new(workspace_id),
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

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionResolveConfig))]
pub async fn resolve_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ResolveConfigBody>,
) -> HttpResponse {
    let flow = Flow::SuperpositionResolveConfig;
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok((org_id, workspace_id)) => (org_id, workspace_id),
        Err(response) => return response,
    };

    let payload = ProxyResolveConfigRequest {
        body: body.into_inner(),
        org_id: Secret::new(org_id),
        workspace_id: Secret::new(workspace_id),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, user, req, _| async move {
            superposition_proxy::resolve_config(state, user, req).await
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

#[instrument(skip_all, fields(flow = ?Flow::SuperpositionListAuditLogs))]
pub async fn list_audit_logs(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListAuditLogs;
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let payload = parse_list_audit_logs_request(org_id, workspace_id, query.into_inner());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, user, req, _| async move {
            superposition_proxy::list_audit_logs(state, user, req).await
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
