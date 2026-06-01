use actix_web::{web, HttpRequest, HttpResponse};
use external_services::superposition::{
    context_put_from_request, parse_datetime, value_to_document, AuditAction, ContextFilterSortOn,
    ContextPutRequest, CreateContextInputBuilder, DimensionMatchStrategy,
    GetResolvedConfigInputBuilder, ListAuditLogsInputBuilder, ListContextsInputBuilder,
    ListDefaultConfigsInputBuilder, ListDimensionsInputBuilder, ResolveConfigBody, SortBy,
};
use router_env::{instrument, logger, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, superposition_proxy},
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

/// Build the Superposition SDK `ListContexts` input builder directly from the
/// raw query parameters.
fn build_list_contexts_input(
    org_id: String,
    workspace_id: String,
    params: Vec<(String, String)>,
) -> ListContextsInputBuilder {
    let mut builder = ListContextsInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id);
    let mut prefix = Vec::new();
    let mut created_by = Vec::new();
    let mut last_modified_by = Vec::new();

    for (key, value) in params {
        match key.as_str() {
            k if k.starts_with("dimension[") => {
                builder = builder.dimension_params(key, value);
            }
            "count" => builder = builder.set_count(value.parse().ok()),
            "page" => builder = builder.set_page(value.parse().ok()),
            "all" => builder = builder.set_all(value.parse().ok()),
            "prefix" => prefix.push(value),
            "sort_on" => {
                builder = builder.set_sort_on(Some(ContextFilterSortOn::from(value.as_str())))
            }
            "sort_by" => builder = builder.set_sort_by(Some(SortBy::from(value.as_str()))),
            "created_by" => created_by.push(value),
            "last_modified_by" => last_modified_by.push(value),
            "plaintext" => builder = builder.set_plaintext(Some(value)),
            "dimension_match_strategy" => {
                builder = builder.set_dimension_match_strategy(Some(DimensionMatchStrategy::from(
                    value.as_str(),
                )))
            }
            _ => {}
        }
    }

    builder
        .set_prefix((!prefix.is_empty()).then_some(prefix))
        .set_created_by((!created_by.is_empty()).then_some(created_by))
        .set_last_modified_by((!last_modified_by.is_empty()).then_some(last_modified_by))
}

/// Build the `ListDefaultConfigs` SDK input builder from typed query parameters.
/// Allowlist-driven paging overrides are applied later in the core handler.
fn build_list_default_configs_input(
    org_id: String,
    workspace_id: String,
    params: SuperpositionListQuery,
) -> ListDefaultConfigsInputBuilder {
    ListDefaultConfigsInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id)
        .set_count(params.count)
        .set_page(params.page)
        .set_all(params.all)
        .set_name(params.name)
}

/// Typed query params shared by the simple Superposition list endpoints
/// (`ListDimensions`, `ListDefaultConfigs`).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SuperpositionListQuery {
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
    pub name: Option<String>,
}

/// Build the `ListDimensions` SDK input builder from typed query parameters.
fn build_list_dimensions_input(
    org_id: String,
    workspace_id: String,
    params: SuperpositionListQuery,
) -> ListDimensionsInputBuilder {
    ListDimensionsInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id)
        .set_count(params.count)
        .set_page(params.page)
        .set_all(params.all)
}

/// Build the `ListAuditLogs` SDK input builder from raw query parameters.
fn build_list_audit_logs_input(
    org_id: String,
    workspace_id: String,
    params: Vec<(String, String)>,
) -> Result<ListAuditLogsInputBuilder, HttpResponse> {
    let bad_date = |field: &str, value: &str| {
        HttpResponse::BadRequest().json(serde_json::json!({
            "error": { "message": format!("invalid {field} format: {value}") }
        }))
    };

    let mut builder = ListAuditLogsInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id);
    let mut table: Vec<String> = Vec::new();
    let mut action: Vec<String> = Vec::new();

    for (key, value) in params {
        match key.as_str() {
            k if k.starts_with("dimension[") => {
                builder = builder.dimension_params(key, value);
            }
            "count" => builder = builder.set_count(value.parse().ok()),
            "page" => builder = builder.set_page(value.parse().ok()),
            "all" => builder = builder.set_all(value.parse().ok()),
            "from_date" => {
                let parsed = parse_datetime(&value).map_err(|_| bad_date("from_date", &value))?;
                builder = builder.set_from_date(Some(parsed));
            }
            "to_date" => {
                let parsed = parse_datetime(&value).map_err(|_| bad_date("to_date", &value))?;
                builder = builder.set_to_date(Some(parsed));
            }
            "table" => table.extend(value.split(',').map(|s| s.trim().to_owned())),
            "action" => action.extend(value.split(',').map(|s| s.trim().to_owned())),
            "username" => builder = builder.set_username(Some(value)),
            "sort_by" => builder = builder.set_sort_by(Some(SortBy::from(value.as_str()))),
            _ => {}
        }
    }

    let action = (!action.is_empty()).then(|| {
        action
            .iter()
            .map(|a| AuditAction::from(a.as_str()))
            .collect()
    });

    Ok(builder
        .set_tables((!table.is_empty()).then_some(table))
        .set_action(action))
}

/// Build the `GetResolvedConfig` SDK input builder from the resolve-config
/// request body.
fn build_resolve_config_input(
    org_id: String,
    workspace_id: String,
    body: ResolveConfigBody,
) -> GetResolvedConfigInputBuilder {
    let mut builder = GetResolvedConfigInputBuilder::default()
        .org_id(org_id)
        .workspace_id(workspace_id);

    for (dimension_key, dimension_value) in body.context {
        builder = builder.context(dimension_key, value_to_document(dimension_value));
    }

    builder
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
    let input = build_list_contexts_input(org_id, workspace_id, query.into_inner());

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
    query: web::Query<SuperpositionListQuery>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListDefaultConfigs;
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let input = build_list_default_configs_input(org_id, workspace_id, query.into_inner());

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
    query: web::Query<SuperpositionListQuery>,
) -> HttpResponse {
    let flow = Flow::SuperpositionListDimensions;
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let input = build_list_dimensions_input(org_id, workspace_id, query.into_inner());

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

    let input = build_resolve_config_input(org_id, workspace_id, body.into_inner());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        move |state, user, _, _| {
            let input = input.clone();
            async move { superposition_proxy::resolve_config(state, user, input).await }
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
    let input = match build_list_audit_logs_input(org_id, workspace_id, query.into_inner()) {
        Ok(input) => input,
        Err(response) => return response,
    };

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
            permission: Permission::MerchantSuperpositionConfigRead,
            allow_connected: true,
            allow_platform: true,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
