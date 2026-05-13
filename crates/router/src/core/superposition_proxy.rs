use common_utils::events::{ApiEventMetric, ApiEventsType};
use external_services::superposition::{
    ContextPutRequest, SuperpositionClient, SuperpositionError,
};
use router_env::logger;

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

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::proxy_get(
        config,
        &req.org_id,
        &req.workspace_id,
        "/context",
        req.params,
    )
    .await
    .map_err(|e| {
        logger::error!(error = ?e, "superposition list_contexts upstream request failed");
        map_superposition_err(e, "Failed to list contexts from Superposition")
    })?;

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
