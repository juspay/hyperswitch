pub use api_models::superposition_proxy::{
    ListAuditLogsRequest, ListContextsRequest, ListDefaultConfigsRequest, ListDimensionsRequest,
    PaginatedListResponse, ProxyCreateContextRequest, ProxyResolveConfigRequest, ResolveConfigBody,
};
use external_services::superposition::{
    audit_log_full_to_value, context_put_from_request, context_response_to_value,
    create_context_output_to_value, datetime_to_string, default_config_response_to_value,
    dimension_response_to_value, document_to_value, map_sdk_error, parse_datetime, value_to_document,
    AuditAction, ContextFilterSortOn, DimensionMatchStrategy, SortBy, SuperpositionError,
};
use router_env::logger;

use crate::{
    core::errors::{self, RouterResponse},
    services::{ApplicationResponse, authentication::UserFromToken},
    SessionState,
};

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

fn filter_by_names(items: &mut Vec<serde_json::Value>, names: Vec<String>) {
    items.retain(|item| {
        let Some(key) = item.get("key").and_then(|v| v.as_str()) else {
            return false;
        };
        names.iter().any(|name| key.starts_with(name.as_str()))
    });
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
    req: ListContextsRequest,
) -> RouterResponse<PaginatedListResponse> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition list_contexts request"
    );

    check_admin_access(&auth, "list_contexts")?;

    let dimension_params_vec: Vec<(String, String)> =
        req.dimension_params.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

    if let Err(validation_error) = auth.require_superposition_context(&dimension_params_vec) {
        logger::warn!(
            user_id = %auth.user_id,
            error = ?validation_error,
            "superposition list_contexts rejected: missing scoping dimension"
        );
        return Err(validation_error);
    }

    if let Err(validation_error) = auth.validate_superposition_params(&dimension_params_vec) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            error = ?validation_error,
            "superposition list_contexts rejected: param validation failed"
        );
        return Err(validation_error);
    }

    let list_contexts_output = state
        .superposition_service
        .superposition_sdk_client()
        .list_contexts()
        .workspace_id(req.workspace_id)
        .org_id(req.org_id)
        .dimension_match_strategy(DimensionMatchStrategy::AnyMatch)
        .set_dimension_params(
            (!req.dimension_params.is_empty()).then_some(req.dimension_params),
        )
        .set_count(req.count)
        .set_page(req.page)
        .set_all(req.all)
        .set_prefix(req.prefix)
        .set_sort_on(req.sort_on.as_deref().map(ContextFilterSortOn::from))
        .set_sort_by(req.sort_by.as_deref().map(SortBy::from))
        .set_created_by(req.created_by)
        .set_last_modified_by(req.last_modified_by)
        .set_plaintext(req.plaintext)
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

    let response = PaginatedListResponse {
        total_pages: list_contexts_output.total_pages(),
        total_items: all_contexts.len() as i32,
        data: all_contexts,
    };

    logger::info!(user_id = %auth.user_id, "superposition list_contexts success");
    Ok(ApplicationResponse::Json(response))
}

pub async fn list_default_configs(
    state: SessionState,
    auth: UserFromToken,
    req: ListDefaultConfigsRequest,
) -> RouterResponse<PaginatedListResponse> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition list_default_configs request"
    );

    check_admin_access(&auth, "list_default_configs")?;

    let name_filter = req.name;
    let fetch_all = name_filter.is_some() || req.all.unwrap_or(false);

    let list_default_configs_output = state
        .superposition_service
        .superposition_sdk_client()
        .list_default_configs()
        .workspace_id(req.workspace_id)
        .org_id(req.org_id)
        .set_count(if fetch_all { None } else { req.count })
        .set_page(if fetch_all { None } else { req.page })
        .set_all(Some(fetch_all))
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

    let mut default_configs: Vec<serde_json::Value> = list_default_configs_output
        .data()
        .iter()
        .map(default_config_response_to_value)
        .collect();

    if let Some(names) = name_filter {
        filter_by_names(&mut default_configs, names);
    }

    let response = PaginatedListResponse {
        total_pages: list_default_configs_output.total_pages(),
        total_items: default_configs.len() as i32,
        data: default_configs,
    };

    logger::info!(user_id = %auth.user_id, "superposition list_default_configs success");
    Ok(ApplicationResponse::Json(response))
}

pub async fn list_dimensions(
    state: SessionState,
    auth: UserFromToken,
    req: ListDimensionsRequest,
) -> RouterResponse<PaginatedListResponse> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition list_dimensions request"
    );

    check_admin_access(&auth, "list_dimensions")?;

    let list_dimensions_output = state
        .superposition_service
        .superposition_sdk_client()
        .list_dimensions()
        .workspace_id(req.workspace_id)
        .org_id(req.org_id)
        .set_count(req.count)
        .set_page(req.page)
        .set_all(req.all)
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

    let response = PaginatedListResponse {
        total_pages: list_dimensions_output.total_pages(),
        total_items: list_dimensions_output.total_items(),
        data: dimensions,
    };

    logger::info!(user_id = %auth.user_id, "superposition list_dimensions success");
    Ok(ApplicationResponse::Json(response))
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
        .workspace_id(req.workspace_id)
        .org_id(req.org_id)
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
    Ok(ApplicationResponse::Json(response))
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
        .workspace_id(req.workspace_id)
        .org_id(req.org_id);

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
    Ok(ApplicationResponse::Json(response))
}

pub async fn list_audit_logs(
    state: SessionState,
    auth: UserFromToken,
    req: ListAuditLogsRequest,
) -> RouterResponse<PaginatedListResponse> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition list_audit_logs request"
    );

    check_admin_access(&auth, "list_audit_logs")?;

    let from_date = req
        .from_date
        .as_deref()
        .map(|s| {
            parse_datetime(s).map_err(|_| {
                error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!("invalid from_date format: {s}"),
                })
            })
        })
        .transpose()?;

    let to_date = req
        .to_date
        .as_deref()
        .map(|s| {
            parse_datetime(s).map_err(|_| {
                error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
                    message: format!("invalid to_date format: {s}"),
                })
            })
        })
        .transpose()?;

    let audit_logs_output = state
        .superposition_service
        .superposition_sdk_client()
        .list_audit_logs()
        .workspace_id(req.workspace_id)
        .org_id(req.org_id)
        .set_count(req.count)
        .set_page(req.page)
        .set_all(req.all)
        .set_from_date(from_date)
        .set_to_date(to_date)
        .set_tables(req.tables)
        .set_action(req.action.map(|actions| actions.iter().map(|a| AuditAction::from(a.as_str())).collect()))
        .set_username(req.username)
        .set_sort_by(req.sort_by.as_deref().map(SortBy::from))
        .set_dimension_params((!req.dimension_params.is_empty()).then_some(req.dimension_params))
        .send()
        .await
        .map_err(|sdk_error| {
            logger::error!(error = ?sdk_error, "superposition list_audit_logs upstream request failed");
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to list audit logs from Superposition",
            )
        })?;

    let audit_logs: Vec<serde_json::Value> = audit_logs_output
        .data()
        .iter()
        .map(audit_log_full_to_value)
        .collect();

    let response = PaginatedListResponse {
        total_pages: audit_logs_output.total_pages(),
        total_items: audit_logs_output.total_items(),
        data: audit_logs,
    };

    logger::info!(user_id = %auth.user_id, "superposition list_audit_logs success");
    Ok(ApplicationResponse::Json(response))
}
