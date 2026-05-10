use external_services::superposition::{SuperpositionClient, SuperpositionError};
use router_env::logger;

use crate::{
    core::errors::{self, RouterResponse},
    services,
    services::authentication::UserFromToken,
    SessionState,
};

fn map_superposition_err(
    err: error_stack::Report<SuperpositionError>,
    context: &'static str,
) -> error_stack::Report<errors::ApiErrorResponse> {
    if let SuperpositionError::BadRequest(msg) = err.current_context() {
        error_stack::report!(errors::ApiErrorResponse::InvalidRequestData {
            message: msg.clone(),
        })
    } else {
        err.change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(context)
    }
}

pub async fn list_contexts(
    state: SessionState,
    auth: UserFromToken,
    params: Vec<(String, String)>,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        query_params = ?params,
        "superposition list_contexts request"
    );

    if !auth.is_superposition_admin() {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            "superposition list_contexts access denied: insufficient role"
        );
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }

    if let Err(e) = auth.require_superposition_context(&params) {
        logger::warn!(
            user_id = %auth.user_id,
            query_params = ?params,
            error = ?e,
            "superposition list_contexts rejected: missing scoping dimension"
        );
        return Err(e);
    }

    if let Err(e) = auth.validate_superposition_params(&params) {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            query_params = ?params,
            error = ?e,
            "superposition list_contexts rejected: param validation failed"
        );
        return Err(e);
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::get_raw(config, "/context", params)
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
    params: Vec<(String, String)>,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        query_params = ?params,
        "superposition list_default_configs request"
    );

    if !auth.is_superposition_admin() {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            "superposition list_default_configs access denied: insufficient role"
        );
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::get_raw(config, "/default-config", params)
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
    params: Vec<(String, String)>,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        query_params = ?params,
        "superposition list_dimensions request"
    );

    if !auth.is_superposition_admin() {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            "superposition list_dimensions access denied: insufficient role"
        );
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::get_raw(config, "/dimension", params)
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
    body: serde_json::Value,
) -> RouterResponse<serde_json::Value> {
    logger::info!(
        user_id = %auth.user_id,
        merchant_id = %auth.merchant_id.get_string_repr(),
        org_id = %auth.org_id.get_string_repr(),
        role_id = %auth.role_id,
        "superposition create_context request"
    );

    if !auth.is_superposition_admin() {
        logger::warn!(
            user_id = %auth.user_id,
            role_id = %auth.role_id,
            "superposition create_context access denied: insufficient role"
        );
        return Err(error_stack::report!(
            errors::ApiErrorResponse::AccessForbidden {
                resource: "superposition".to_string(),
            }
        ));
    }

    ["context", "override", "change_reason"]
        .iter()
        .try_for_each(|&field| {
            if body.get(field).is_none() {
                logger::warn!(
                    user_id = %auth.user_id,
                    missing_field = field,
                    "superposition create_context rejected: missing required field"
                );
                Err(error_stack::report!(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("missing required field: {field}"),
                    }
                ))
            } else {
                Ok(())
            }
        })?;

    if let Some(context_value) = body.get("context") {
        if let Err(e) = auth.validate_superposition_context_body(context_value) {
            logger::warn!(
                user_id = %auth.user_id,
                role_id = %auth.role_id,
                context = ?context_value,
                error = ?e,
                "superposition create_context rejected: context dimension validation failed"
            );
            return Err(e);
        }
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::put_raw(config, "/context", body)
        .await
        .map_err(|e| {
            logger::error!(error = ?e, "superposition create_context upstream request failed");
            map_superposition_err(e, "Failed to create context in Superposition")
        })?;

    logger::info!(user_id = %auth.user_id, "superposition create_context success");
    Ok(services::ApplicationResponse::Json(response))
}
