use common_utils::events::{ApiEventMetric, ApiEventsType};
use external_services::superposition::{
    ContextPutRequest, SuperpositionClient, SuperpositionError,
};
use futures::future::join_all;
use router_env::logger;
use serde_json::Map;

use crate::{
    core::errors::{self, RouterResponse},
    services,
    services::authentication::UserFromToken,
    SessionState,
};

#[derive(Debug, serde::Serialize)]
pub struct ProxyListRequest {
    pub params: Vec<(String, String)>,
    pub org_id: String,
    pub workspace_id: String,
}

impl ApiEventMetric for ProxyListRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(serde::Serialize)]
pub struct ProxyCreateContextRequest {
    pub body: ContextPutRequest,
    pub org_id: String,
    pub workspace_id: String,
}

impl std::fmt::Debug for ProxyCreateContextRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyCreateContextRequest")
            .field("org_id", &self.org_id)
            .field("workspace_id", &self.workspace_id)
            .finish()
    }
}

impl ApiEventMetric for ProxyCreateContextRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ResolveConfigBody {
    pub context: Map<String, serde_json::Value>,
}

#[derive(Debug, serde::Serialize)]
pub struct ProxyResolveConfigRequest {
    pub body: ResolveConfigBody,
    pub org_id: String,
    pub workspace_id: String,
}

impl ApiEventMetric for ProxyResolveConfigRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Miscellaneous)
    }
}

fn check_admin_access(
    auth: &UserFromToken,
    operation: &'static str,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    if !auth.is_merchant_or_org_admin() {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            operation = operation,
            "superposition access denied: insufficient role"
        );
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }
    Ok(())
}

/// Generate all non-empty subsets of dimension query params.
/// For dims [O, M, P] produces 7 subsets: O, M, P, OM, OP, MP, OMP.
fn generate_dim_subsets(dim_params: &[(String, String)]) -> Vec<Vec<(String, String)>> {
    let n = dim_params.len();
    let mut subsets = Vec::with_capacity((1usize << n).saturating_sub(1));
    for mask in 1u32..(1u32 << n) {
        let subset = dim_params
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, p)| p.clone())
            .collect();
        subsets.push(subset);
    }
    subsets
}

fn map_superposition_err(
    err: error_stack::Report<SuperpositionError>,
    context: &'static str,
) -> error_stack::Report<errors::ApiErrorResponse> {
    match err.current_context() {
        SuperpositionError::BadRequest(msg) => {
            error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                message: msg.clone(),
            })
        }
        SuperpositionError::NotFound(_) => {
            error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "The specified org_id or workspace_id was not found in Superposition"
                    .to_string(),
            })
        }
        _ => err
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(context),
    }
}

pub async fn list_contexts(
    state: SessionState,
    auth: UserFromToken,
    req: ProxyListRequest,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        query_params = ?req.params,
        "superposition list_contexts request"
    );

    check_admin_access(&auth, "list_contexts")?;

    if let Err(e) = auth.require_superposition_context(&req.params) {
        logger::warn!(
            user_id = %auth.user_id,
            query_params = ?req.params,
            error = ?e,
            "superposition list_contexts rejected: missing scoping dimension"
        );
        return Err(e);
    }

    if let Err(e) = auth.validate_superposition_params(&req.params) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            query_params = ?req.params,
            error = ?e,
            "superposition list_contexts rejected: param validation failed"
        );
        return Err(e);
    }

    let (dim_params, other_params): (Vec<_>, Vec<_>) = req
        .params
        .into_iter()
        .partition(|(k, _)| k.starts_with("dimension["));

    let config = state.conf.superposition.get_inner();

    let subset_params_list: Vec<Vec<(String, String)>> = generate_dim_subsets(&dim_params)
        .into_iter()
        .map(|subset| {
            let mut params = other_params.clone();
            params.extend(subset);
            params
        })
        .collect();

    let call_futures: Vec<_> = subset_params_list
        .into_iter()
        .map(|params| {
            SuperpositionClient::proxy_get(
                config,
                &req.org_id,
                &req.workspace_id,
                "/context",
                params,
            )
        })
        .collect();

    let results = join_all(call_futures).await;

    let mut seen_ids = std::collections::HashSet::new();
    let mut all_contexts: Vec<serde_json::Value> = Vec::new();

    for result in results {
        let response = result.map_err(|e| {
            logger::error!(error = ?e, "superposition list_contexts upstream request failed");
            map_superposition_err(e, "Failed to list contexts from Superposition")
        })?;
        if let Some(data) = response.get("data").and_then(|d| d.as_array()) {
            for ctx in data {
                if let Some(id) = ctx.get("id").and_then(|id| id.as_str()) {
                    if seen_ids.insert(id.to_string()) {
                        all_contexts.push(ctx.clone());
                    }
                }
            }
        }
    }

    if auth.role_id == crate::consts::user_role::ROLE_ID_MERCHANT_ADMIN {
        let scoped_merchant_id = auth.merchant_id.get_string_repr().to_string();
        let scoped_profile_id = auth.profile_id.get_string_repr().to_string();

        all_contexts.retain(|ctx| {
            let Some(dims) = ctx.get("value").and_then(|v| v.as_object()) else {
                return true;
            };

            let dim_str = |key: &str| dims.get(key).and_then(|v| v.as_str());

            if dim_str("merchant_id").is_some_and(|v| v != scoped_merchant_id) {
                return false;
            }
            if dim_str("processor_merchant_id").is_some_and(|v| v != scoped_merchant_id) {
                return false;
            }
            if dim_str("provider_merchant_id").is_some_and(|v| v != scoped_merchant_id) {
                return false;
            }
            if dim_str("profile_id").is_some_and(|v| v != scoped_profile_id) {
                return false;
            }
            true
        });
    }

    let count = all_contexts.len() as i64;
    let response = serde_json::json!({
        "total_pages": 1,
        "total_items": count,
        "data": all_contexts,
    });

    logger::info!(user_id = %auth.user_id, "superposition list_contexts success");
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn list_default_configs(
    state: SessionState,
    auth: UserFromToken,
    req: ProxyListRequest,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        query_params = ?req.params,
        "superposition list_default_configs request"
    );

    check_admin_access(&auth, "list_default_configs")?;

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::proxy_get(
        config,
        &req.org_id,
        &req.workspace_id,
        "/default-config",
        req.params,
    )
    .await
    .map_err(|e| {
        logger::error!(error = ?e, "superposition list_default_configs upstream request failed");
        map_superposition_err(e, "Failed to list default configs from Superposition")
    })?;

    logger::info!(user_id = %auth.user_id, "superposition list_default_configs success");
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn list_dimensions(
    state: SessionState,
    auth: UserFromToken,
    req: ProxyListRequest,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        query_params = ?req.params,
        "superposition list_dimensions request"
    );

    check_admin_access(&auth, "list_dimensions")?;

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::proxy_get(
        config,
        &req.org_id,
        &req.workspace_id,
        "/dimension",
        req.params,
    )
    .await
    .map_err(|e| {
        logger::error!(error = ?e, "superposition list_dimensions upstream request failed");
        map_superposition_err(e, "Failed to list dimensions from Superposition")
    })?;

    logger::info!(user_id = %auth.user_id, "superposition list_dimensions success");
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn create_context(
    state: SessionState,
    auth: UserFromToken,
    req: ProxyCreateContextRequest,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition create_context request"
    );

    check_admin_access(&auth, "create_context")?;

    let context_json = serde_json::to_value(&req.body.context).map_err(|e| {
        error_stack::report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("failed to serialize context: {e}"))
    })?;

    if let Err(e) = auth.validate_superposition_context_body(&context_json) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            context = ?context_json,
            error = ?e,
            "superposition create_context rejected: context dimension validation failed"
        );
        return Err(e);
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::proxy_put(
        config,
        &req.org_id,
        &req.workspace_id,
        "/context",
        &req.body,
    )
    .await
    .map_err(|e| {
        logger::error!(error = ?e, "superposition create_context upstream request failed");
        map_superposition_err(e, "Failed to create context in Superposition")
    })?;

    logger::info!(user_id = %auth.user_id, "superposition create_context success");
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn resolve_config(
    state: SessionState,
    auth: UserFromToken,
    req: ProxyResolveConfigRequest,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition resolve_config request"
    );

    check_admin_access(&auth, "resolve_config")?;

    let context_json = serde_json::Value::Object(req.body.context.clone());
    if let Err(e) = auth.validate_superposition_context_body(&context_json) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            context = ?context_json,
            error = ?e,
            "superposition resolve_config rejected: context dimension validation failed"
        );
        return Err(e);
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::proxy_post(
        config,
        &req.org_id,
        &req.workspace_id,
        "/config/resolve",
        &req.body,
    )
    .await
    .map_err(|e| {
        logger::error!(error = ?e, "superposition resolve_config upstream request failed");
        map_superposition_err(e, "Failed to resolve config from Superposition")
    })?;

    logger::info!(user_id = %auth.user_id, "superposition resolve_config success");
    Ok(services::ApplicationResponse::Json(response))
}
