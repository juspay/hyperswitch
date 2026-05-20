use common_utils::events::{ApiEventMetric, ApiEventsType};
use external_services::superposition::{
    append_query_params, build_query_string, context_put_from_request, context_response_to_value,
    create_context_output_to_value, datetime_to_string, default_config_response_to_value,
    dimension_response_to_value, document_to_value, map_sdk_error, value_to_document,
    ContextPutRequest, DimensionMatchStrategy, SuperpositionError,
};
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

fn map_superposition_err(
    superposition_error: error_stack::Report<SuperpositionError>,
    context: &'static str,
) -> error_stack::Report<errors::ApiErrorResponse> {
    match superposition_error.current_context() {
        SuperpositionError::BadRequest(error_message) => {
            error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                message: error_message.clone(),
            })
        }
        SuperpositionError::NotFound(_) => {
            error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "The specified org_id or workspace_id was not found in Superposition"
                    .to_string(),
            })
        }
        _ => superposition_error
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

    if let Err(validation_error) = auth.require_superposition_context(&req.params) {
        logger::warn!(
            user_id = %auth.user_id,
            query_params = ?req.params,
            error = ?validation_error,
            "superposition list_contexts rejected: missing scoping dimension"
        );
        return Err(validation_error);
    }

    if let Err(validation_error) = auth.validate_superposition_params(&req.params) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            query_params = ?req.params,
            error = ?validation_error,
            "superposition list_contexts rejected: param validation failed"
        );
        return Err(validation_error);
    }

    let encoded_query_params = build_query_string(&req.params);

    let list_contexts_output = state
        .superposition_service
        .superposition_sdk_client()
        .list_contexts()
        .workspace_id(req.workspace_id.clone())
        .org_id(req.org_id.clone())
        .dimension_match_strategy(DimensionMatchStrategy::AnyMatch)
        .customize()
        .mutate_request(move |request| append_query_params(request, &encoded_query_params))
        .send()
        .await
        .map_err(|sdk_error| {
            logger::error!(error = ?sdk_error, "superposition list_contexts upstream request failed");
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to list contexts from Superposition",
            )
        })?;

    let mut all_contexts: Vec<serde_json::Value> = list_contexts_output
        .data()
        .iter()
        .map(context_response_to_value)
        .collect();

    if auth.role_id == crate::consts::user_role::ROLE_ID_MERCHANT_ADMIN {
        let scoped_merchant_id = auth.merchant_id.get_string_repr().to_string();
        let scoped_profile_id = auth.profile_id.get_string_repr().to_string();

        all_contexts.retain(|context| {
            let Some(context_dimensions) = context.get("value").and_then(|v| v.as_object()) else {
                return true;
            };

            let get_dimension_value =
                |key: &str| context_dimensions.get(key).and_then(|v| v.as_str());

            if get_dimension_value("merchant_id").is_some_and(|v| v != scoped_merchant_id) {
                return false;
            }
            if get_dimension_value("processor_merchant_id").is_some_and(|v| v != scoped_merchant_id)
            {
                return false;
            }
            if get_dimension_value("provider_merchant_id").is_some_and(|v| v != scoped_merchant_id)
            {
                return false;
            }
            if get_dimension_value("profile_id").is_some_and(|v| v != scoped_profile_id) {
                return false;
            }
            true
        });
    }

    let total_items = all_contexts.len() as i64;
    let response = serde_json::json!({
        "total_pages": list_contexts_output.total_pages(),
        "total_items": total_items,
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

    let encoded_query_params = build_query_string(&req.params);

    let list_default_configs_output = state
        .superposition_service
        .superposition_sdk_client()
        .list_default_configs()
        .workspace_id(req.workspace_id.clone())
        .org_id(req.org_id.clone())
        .customize()
        .mutate_request(move |request| append_query_params(request, &encoded_query_params))
        .send()
        .await
        .map_err(|sdk_error| {
            logger::error!(
                error = ?sdk_error,
                "superposition list_default_configs upstream request failed"
            );
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to list default configs from Superposition",
            )
        })?;

    let default_configs: Vec<serde_json::Value> = list_default_configs_output
        .data()
        .iter()
        .map(default_config_response_to_value)
        .collect();

    let response = serde_json::json!({
        "total_pages": list_default_configs_output.total_pages(),
        "total_items": list_default_configs_output.total_items(),
        "data": default_configs,
    });

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

    let encoded_query_params = build_query_string(&req.params);

    let list_dimensions_output = state
        .superposition_service
        .superposition_sdk_client()
        .list_dimensions()
        .workspace_id(req.workspace_id.clone())
        .org_id(req.org_id.clone())
        .customize()
        .mutate_request(move |request| append_query_params(request, &encoded_query_params))
        .send()
        .await
        .map_err(|sdk_error| {
            logger::error!(
                error = ?sdk_error,
                "superposition list_dimensions upstream request failed"
            );
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to list dimensions from Superposition",
            )
        })?;

    let dimensions: Vec<serde_json::Value> = list_dimensions_output
        .data()
        .iter()
        .map(dimension_response_to_value)
        .collect();

    let response = serde_json::json!({
        "total_pages": list_dimensions_output.total_pages(),
        "total_items": list_dimensions_output.total_items(),
        "data": dimensions,
    });

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

    let context_json = serde_json::to_value(&req.body.context).map_err(|serialize_error| {
        error_stack::report!(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!("failed to serialize context: {serialize_error}"))
    })?;

    if let Err(validation_error) = auth.validate_superposition_context_body(&context_json) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            context = ?context_json,
            error = ?validation_error,
            "superposition create_context rejected: context dimension validation failed"
        );
        return Err(validation_error);
    }

    let context_put = context_put_from_request(&req.body).map_err(|build_error| {
        logger::error!(error = ?build_error, "superposition create_context failed to build ContextPut");
        build_error.change_context(errors::ApiErrorResponse::InternalServerError)
    })?;

    let created_context = state
        .superposition_service
        .superposition_sdk_client()
        .create_context()
        .workspace_id(req.workspace_id.clone())
        .org_id(req.org_id.clone())
        .request(context_put)
        .send()
        .await
        .map_err(|sdk_error| {
            logger::error!(error = ?sdk_error, "superposition create_context upstream request failed");
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to create context in Superposition",
            )
        })?;

    let response = create_context_output_to_value(&created_context);

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
    if let Err(validation_error) = auth.validate_superposition_context_body(&context_json) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            context = ?context_json,
            error = ?validation_error,
            "superposition resolve_config rejected: context dimension validation failed"
        );
        return Err(validation_error);
    }

    let mut resolved_config_builder = state
        .superposition_service
        .superposition_sdk_client()
        .get_resolved_config()
        .workspace_id(req.workspace_id.clone())
        .org_id(req.org_id.clone());

    for (dimension_key, dimension_value) in &req.body.context {
        resolved_config_builder = resolved_config_builder.context(
            dimension_key.clone(),
            value_to_document(dimension_value.clone()),
        );
    }

    let resolved_config = resolved_config_builder.send().await.map_err(|sdk_error| {
        logger::error!(error = ?sdk_error, "superposition resolve_config upstream request failed");
        map_superposition_err(
            error_stack::report!(map_sdk_error(sdk_error)),
            "Failed to resolve config from Superposition",
        )
    })?;

    let response = serde_json::json!({
        "config": document_to_value(resolved_config.config().clone()),
        "version": resolved_config.version(),
        "last_modified": datetime_to_string(resolved_config.last_modified()),
        "audit_id": resolved_config.audit_id(),
    });

    logger::info!(user_id = %auth.user_id, "superposition resolve_config success");
    Ok(services::ApplicationResponse::Json(response))
}
