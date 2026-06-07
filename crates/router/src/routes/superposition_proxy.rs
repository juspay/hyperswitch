use actix_web::{web, HttpRequest, HttpResponse};
use external_services::superposition::{
    context_put_from_request, parse_datetime, value_to_document, AuditAction, ContextFilterSortOn,
    ContextPutRequest, CreateContextInputBuilder, DateTime, DimensionMatchStrategy,
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

/// Typed `ListContexts` query params, parsed from the raw key/value pairs
#[derive(Debug, Default)]
struct ListContextsQuery {
    count: Option<i32>,
    page: Option<i32>,
    all: Option<bool>,
    prefix: Vec<String>,
    sort_on: Option<ContextFilterSortOn>,
    sort_by: Option<SortBy>,
    created_by: Vec<String>,
    last_modified_by: Vec<String>,
    plaintext: Option<String>,
    dimension_match_strategy: Option<DimensionMatchStrategy>,
    dimension_params: Vec<(String, String)>,
}

impl From<Vec<(String, String)>> for ListContextsQuery {
    fn from(params: Vec<(String, String)>) -> Self {
        let first_value = |name: &str| {
            params
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        let all_values = |name: &str| {
            params
                .iter()
                .filter(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .collect::<Vec<_>>()
        };

        Self {
            count: first_value("count").and_then(|v| v.parse().ok()),
            page: first_value("page").and_then(|v| v.parse().ok()),
            all: first_value("all").and_then(|v| v.parse().ok()),
            prefix: all_values("prefix"),
            sort_on: first_value("sort_on").map(|v| ContextFilterSortOn::from(v.as_str())),
            sort_by: first_value("sort_by").map(|v| SortBy::from(v.as_str())),
            created_by: all_values("created_by"),
            last_modified_by: all_values("last_modified_by"),
            plaintext: first_value("plaintext"),
            dimension_match_strategy: first_value("dimension_match_strategy")
                .map(|v| DimensionMatchStrategy::from(v.as_str())),
            dimension_params: params
                .iter()
                .filter(|(k, _)| k.starts_with("dimension["))
                .cloned()
                .collect(),
        }
    }
}

impl ListContextsQuery {
    /// Build the `ListContexts` SDK input from the typed query.
    fn into_input(self, org_id: String, workspace_id: String) -> ListContextsInputBuilder {
        let mut builder = ListContextsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all)
            .set_sort_on(self.sort_on)
            .set_sort_by(self.sort_by)
            .set_plaintext(self.plaintext)
            .set_dimension_match_strategy(self.dimension_match_strategy)
            .set_prefix((!self.prefix.is_empty()).then_some(self.prefix))
            .set_created_by((!self.created_by.is_empty()).then_some(self.created_by))
            .set_last_modified_by(
                (!self.last_modified_by.is_empty()).then_some(self.last_modified_by),
            );

        for (key, value) in self.dimension_params {
            builder = builder.dimension_params(key, value);
        }

        builder
    }
}

/// Typed query params shared by `ListDimensions` and `ListDefaultConfigs`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SuperpositionListQuery {
    pub count: Option<i32>,
    pub page: Option<i32>,
    pub all: Option<bool>,
    pub name: Option<String>,
}

impl SuperpositionListQuery {
    /// Build the `ListDimensions` SDK input from the typed query.
    fn into_dimensions_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> ListDimensionsInputBuilder {
        ListDimensionsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all)
    }

    /// Build the `ListDefaultConfigs` SDK input from the typed query.
    fn into_default_configs_input(
        self,
        org_id: String,
        workspace_id: String,
    ) -> ListDefaultConfigsInputBuilder {
        ListDefaultConfigsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all)
            .set_name(self.name)
    }
}

/// Typed `ListAuditLogs` query params, parsed from the raw key/value pairs.
#[derive(Debug, Default)]
struct ListAuditLogsQuery {
    count: Option<i32>,
    page: Option<i32>,
    all: Option<bool>,
    from_date: Option<DateTime>,
    to_date: Option<DateTime>,
    table: Vec<String>,
    action: Vec<String>,
    username: Option<String>,
    sort_by: Option<SortBy>,
    dimension_params: Vec<(String, String)>,
}

impl TryFrom<Vec<(String, String)>> for ListAuditLogsQuery {
    type Error = HttpResponse;

    fn try_from(params: Vec<(String, String)>) -> Result<Self, Self::Error> {
        let first_value = |name: &str| {
            params
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        // `table`/`action` accept comma-separated values across one or more keys.
        let csv_values = |name: &str| {
            params
                .iter()
                .filter(|(k, _)| k == name)
                .flat_map(|(_, v)| v.split(',').map(|s| s.trim().to_owned()))
                .collect::<Vec<_>>()
        };
        let parse_date = |name: &str| {
            first_value(name)
                .map(|v| {
                    parse_datetime(&v).map_err(|_| {
                        HttpResponse::BadRequest().json(serde_json::json!({
                            "error": { "message": format!("invalid {name} format: {v}") }
                        }))
                    })
                })
                .transpose()
        };

        Ok(Self {
            count: first_value("count").and_then(|v| v.parse().ok()),
            page: first_value("page").and_then(|v| v.parse().ok()),
            all: first_value("all").and_then(|v| v.parse().ok()),
            from_date: parse_date("from_date")?,
            to_date: parse_date("to_date")?,
            table: csv_values("table"),
            action: csv_values("action"),
            username: first_value("username"),
            sort_by: first_value("sort_by").map(|v| SortBy::from(v.as_str())),
            dimension_params: params
                .iter()
                .filter(|(k, _)| k.starts_with("dimension["))
                .cloned()
                .collect(),
        })
    }
}

impl ListAuditLogsQuery {
    /// Build the `ListAuditLogs` SDK input from the typed query.
    fn into_input(self, org_id: String, workspace_id: String) -> ListAuditLogsInputBuilder {
        let action = (!self.action.is_empty()).then(|| {
            self.action
                .iter()
                .map(|a| AuditAction::from(a.as_str()))
                .collect()
        });

        let mut builder = ListAuditLogsInputBuilder::default()
            .org_id(org_id)
            .workspace_id(workspace_id)
            .set_count(self.count)
            .set_page(self.page)
            .set_all(self.all)
            .set_from_date(self.from_date)
            .set_to_date(self.to_date)
            .set_username(self.username)
            .set_sort_by(self.sort_by)
            .set_tables((!self.table.is_empty()).then_some(self.table))
            .set_action(action);

        for (key, value) in self.dimension_params {
            builder = builder.dimension_params(key, value);
        }

        builder
    }
}

/// Build the `GetResolvedConfig` SDK input from the resolve-config body.
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
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
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
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
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
            permission: Permission::ProfileSuperpositionConfigWrite,
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
    let (org_id, workspace_id) = match extract_proxy_headers(&req) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let params = match ListAuditLogsQuery::try_from(query.into_inner()) {
        Ok(params) => params,
        Err(response) => return response,
    };
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
