pub use api_models::superposition_proxy::{
    AuditLogResponse, ContextResponse, DefaultConfigResponse, DimensionResponse,
    PaginatedListResponse, ResolveConfigResponse,
};
use external_services::superposition::{
    audit_log_full_to_struct, context_response_to_struct, create_context_output_to_struct,
    datetime_to_string, default_config_response_to_struct, dimension_response_to_struct,
    doc_map_to_json, document_to_value, map_sdk_error, CreateContextInputBuilder,
    GetResolvedConfigInputBuilder, ListAuditLogsInputBuilder, ListContextsInputBuilder,
    ListDefaultConfigsInputBuilder, ListDimensionsInputBuilder, SuperpositionError,
};
use router_env::logger;

use crate::{
    consts::user_role::{ROLE_ID_MERCHANT_ADMIN, ROLE_ID_PROFILE_ADMIN},
    core::errors::{self, RouterResponse},
    services::{authentication::UserFromToken, ApplicationResponse},
    SessionState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
enum ScopingDimension {
    OrganizationId,
    MerchantId,
    ProfileId,
    ProviderMerchantId,
    ProcessorMerchantId,
}

impl ScopingDimension {
    fn from_context_key(key: &str) -> Option<Self> {
        match key {
            "organization_id" => Some(Self::OrganizationId),
            "merchant_id" => Some(Self::MerchantId),
            "profile_id" => Some(Self::ProfileId),
            "provider_merchant_id" => Some(Self::ProviderMerchantId),
            "processor_merchant_id" => Some(Self::ProcessorMerchantId),
            _ => None,
        }
    }

    fn from_dimension_param(key: &str) -> Option<Self> {
        key.strip_prefix("dimension[")
            .and_then(|s| s.strip_suffix(']'))
            .and_then(Self::from_context_key)
    }

    fn expected_value(self, auth: &UserFromToken) -> &str {
        match self {
            Self::OrganizationId => auth.org_id.get_string_repr(),
            Self::MerchantId | Self::ProviderMerchantId | Self::ProcessorMerchantId => {
                auth.merchant_id.get_string_repr()
            }
            Self::ProfileId => auth.profile_id.get_string_repr(),
        }
    }
}

fn validate_superposition_params(
    params: &[(String, String)],
    auth: &UserFromToken,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let unauthorized = || {
        error_stack::report!(errors::ApiErrorResponse::AccessForbidden {
            resource: "superposition".to_string(),
        })
    };
    for (key, value) in params {
        if let Some(dimension) = ScopingDimension::from_dimension_param(key) {
            if value != dimension.expected_value(auth) {
                return Err(unauthorized());
            }
        }
    }
    Ok(())
}

fn require_superposition_context(
    params: &[(String, String)],
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let has_scoping_dimension = params
        .iter()
        .any(|(k, _)| ScopingDimension::from_dimension_param(k).is_some());
    if !has_scoping_dimension {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "at least one dimension filter (organization_id, provider_merchant_id, processor_merchant_id, merchant_id, or profile_id) is required".to_string(),
            }
        ));
    }
    Ok(())
}

fn validate_superposition_context_body(
    context: &serde_json::Value,
    auth: &UserFromToken,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let Some(context_obj) = context.as_object() else {
        return Ok(());
    };
    let has_scoping_dim = context_obj
        .keys()
        .any(|k| ScopingDimension::from_context_key(k).is_some());
    if !has_scoping_dim {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: "context must contain at least one of: organization_id, profile_id, provider_merchant_id, processor_merchant_id".to_string(),
            }
        ));
    }
    let is_merchant_admin_role = auth.role_id == ROLE_ID_MERCHANT_ADMIN;
    let is_profile_admin_role = auth.role_id == ROLE_ID_PROFILE_ADMIN;
    let has_merchant_level_dim = context_obj.contains_key("merchant_id")
        || context_obj.contains_key("profile_id")
        || context_obj.contains_key("processor_merchant_id")
        || context_obj.contains_key("provider_merchant_id");
    if is_merchant_admin_role
        && context_obj.contains_key("organization_id")
        && !has_merchant_level_dim
    {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }
    // Profile admin: body must carry profile_id (no org-only/merchant-only contexts).
    if is_profile_admin_role && !context_obj.contains_key("profile_id") {
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }
    let params = context_obj
        .iter()
        .filter_map(|(k, v)| {
            v.as_str()
                .map(|s| (format!("dimension[{k}]"), s.to_owned()))
        })
        .collect::<Vec<_>>();
    validate_superposition_params(&params, auth)
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
    input: ListContextsInputBuilder,
) -> RouterResponse<PaginatedListResponse<ContextResponse>> {
    logger::info!(
        user_id = %auth.user_id,
        role_id = %auth.role_id,
        "superposition list_contexts request"
    );

    let dimension_params_vec: Vec<(String, String)> = input
        .get_dimension_params()
        .iter()
        .flat_map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())))
        .collect();

    if let Err(validation_error) = require_superposition_context(&dimension_params_vec) {
        logger::warn!(
            user_id = %auth.user_id,
            error = ?validation_error,
            "superposition list_contexts rejected: missing scoping dimension"
        );
        return Err(validation_error);
    }

    if let Err(validation_error) = validate_superposition_params(&dimension_params_vec, &auth) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            error = ?validation_error,
            "superposition list_contexts rejected: param validation failed"
        );
        return Err(validation_error);
    }
    let list_contexts_output = input
        .send_with(state.superposition_service.superposition_sdk_client())
        .await
        .map_err(|sdk_error| {
            logger::error!(error = ?sdk_error, "superposition list_contexts upstream request failed");
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to list contexts from Superposition",
            )
        })?;

    let data: Vec<ContextResponse> = list_contexts_output
        .data()
        .iter()
        .map(context_response_to_struct)
        .collect();

    let response = PaginatedListResponse {
        total_pages: list_contexts_output.total_pages(),
        total_items: list_contexts_output.total_items(),
        data,
    };

    logger::info!(user_id = %auth.user_id, "superposition list_contexts success");
    Ok(ApplicationResponse::Json(response))
}

pub async fn list_default_configs(
    state: SessionState,
    auth: UserFromToken,
    input: ListDefaultConfigsInputBuilder,
) -> RouterResponse<PaginatedListResponse<DefaultConfigResponse>> {
    logger::info!(
        user_id = %auth.user_id,
        role_id = %auth.role_id,
        "superposition list_default_configs request"
    );

    let list_default_configs_output = input
        .send_with(state.superposition_service.superposition_sdk_client())
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

    let default_configs: Vec<DefaultConfigResponse> = list_default_configs_output
        .data()
        .iter()
        .map(default_config_response_to_struct)
        .collect();

    let response = PaginatedListResponse {
        total_pages: list_default_configs_output.total_pages(),
        total_items: list_default_configs_output.total_items(),
        data: default_configs,
    };

    logger::info!(user_id = %auth.user_id, "superposition list_default_configs success");
    Ok(ApplicationResponse::Json(response))
}

pub async fn list_dimensions(
    state: SessionState,
    auth: UserFromToken,
    input: ListDimensionsInputBuilder,
) -> RouterResponse<PaginatedListResponse<DimensionResponse>> {
    logger::info!(
        user_id = %auth.user_id,
        role_id = %auth.role_id,
        "superposition list_dimensions request"
    );

    let list_dimensions_output = input
        .send_with(state.superposition_service.superposition_sdk_client())
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

    let dimensions: Vec<DimensionResponse> = list_dimensions_output
        .data()
        .iter()
        .map(dimension_response_to_struct)
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
    input: CreateContextInputBuilder,
) -> RouterResponse<ContextResponse> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition create_context request"
    );

    // Read context dims off the SDK request for auth-scoped validation.
    let context_json = input
        .get_request()
        .as_ref()
        .map(|context_put| doc_map_to_json(context_put.context()))
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    if let Err(validation_error) = validate_superposition_context_body(&context_json, &auth) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            context = ?context_json,
            error = ?validation_error,
            "superposition create_context rejected: context dimension validation failed"
        );
        return Err(validation_error);
    }

    let created_context = input
        .send_with(state.superposition_service.superposition_sdk_client())
        .await
        .map_err(|sdk_error| {
            logger::error!(error = ?sdk_error, "superposition create_context upstream request failed");
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to create context in Superposition",
            )
        })?;

    let response = create_context_output_to_struct(&created_context);

    logger::info!(user_id = %auth.user_id, "superposition create_context success");
    Ok(ApplicationResponse::Json(response))
}

pub async fn resolve_config(
    state: SessionState,
    auth: UserFromToken,
    input: GetResolvedConfigInputBuilder,
) -> RouterResponse<ResolveConfigResponse> {
    logger::info!(
        user_id = %auth.user_id,
        role_id = %auth.role_id,
        "superposition resolve_config request"
    );

    // Read context dims off the SDK request for auth-scoped validation.
    let context_json = input
        .get_context()
        .as_ref()
        .map(doc_map_to_json)
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    if let Err(validation_error) = validate_superposition_context_body(&context_json, &auth) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            context = ?context_json,
            error = ?validation_error,
            "superposition resolve_config rejected: context dimension validation failed"
        );
        return Err(validation_error);
    }

    let resolved_config = input
        .send_with(state.superposition_service.superposition_sdk_client())
        .await
        .map_err(|sdk_error| {
        logger::error!(error = ?sdk_error, "superposition resolve_config upstream request failed");
        map_superposition_err(
            error_stack::report!(map_sdk_error(sdk_error)),
            "Failed to resolve config from Superposition",
        )
    })?;

    let config_value = document_to_value(resolved_config.config().clone());

    let response = ResolveConfigResponse {
        config: config_value,
        version: resolved_config.version().to_owned(),
        last_modified: datetime_to_string(resolved_config.last_modified()),
        audit_id: resolved_config.audit_id().map(str::to_owned),
    };

    logger::info!(user_id = %auth.user_id, "superposition resolve_config success");
    Ok(ApplicationResponse::Json(response))
}

pub async fn list_audit_logs(
    state: SessionState,
    auth: UserFromToken,
    input: ListAuditLogsInputBuilder,
) -> RouterResponse<PaginatedListResponse<AuditLogResponse>> {
    logger::info!(
        user_id = %auth.user_id,
        role_id = %auth.role_id,
        "superposition list_audit_logs request"
    );

    let audit_logs_output = input
        .send_with(state.superposition_service.superposition_sdk_client())
        .await
        .map_err(|sdk_error| {
            logger::error!(error = ?sdk_error, "superposition list_audit_logs upstream request failed");
            map_superposition_err(
                error_stack::report!(map_sdk_error(sdk_error)),
                "Failed to list audit logs from Superposition",
            )
        })?;

    let audit_logs: Vec<AuditLogResponse> = audit_logs_output
        .data()
        .iter()
        .map(audit_log_full_to_struct)
        .collect();

    let response = PaginatedListResponse {
        total_pages: audit_logs_output.total_pages(),
        total_items: audit_logs_output.total_items(),
        data: audit_logs,
    };

    logger::info!(user_id = %auth.user_id, "superposition list_audit_logs success");
    Ok(ApplicationResponse::Json(response))
}
