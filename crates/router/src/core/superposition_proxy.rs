use external_services::superposition::{SuperpositionClient, SuperpositionError};

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
    if !auth.is_superposition_admin() {
        return Err(error_stack::report!(errors::ApiErrorResponse::Unauthorized));
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::get_raw(config, "/context", params)
        .await
        .map_err(|e| map_superposition_err(e, "Failed to list contexts from Superposition"))?;

    Ok(services::ApplicationResponse::Json(response))
}

pub async fn list_default_configs(
    state: SessionState,
    auth: UserFromToken,
    params: Vec<(String, String)>,
) -> RouterResponse<serde_json::Value> {
    if !auth.is_superposition_admin() {
        return Err(error_stack::report!(errors::ApiErrorResponse::Unauthorized));
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::get_raw(config, "/default-config", params)
        .await
        .map_err(|e| {
            map_superposition_err(e, "Failed to list default configs from Superposition")
        })?;

    Ok(services::ApplicationResponse::Json(response))
}

pub async fn list_dimensions(
    state: SessionState,
    auth: UserFromToken,
    params: Vec<(String, String)>,
) -> RouterResponse<serde_json::Value> {
    if !auth.is_superposition_admin() {
        return Err(error_stack::report!(errors::ApiErrorResponse::Unauthorized));
    }

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::get_raw(config, "/dimension", params)
        .await
        .map_err(|e| map_superposition_err(e, "Failed to list dimensions from Superposition"))?;

    Ok(services::ApplicationResponse::Json(response))
}

pub async fn create_context(
    state: SessionState,
    auth: UserFromToken,
    body: serde_json::Value,
) -> RouterResponse<serde_json::Value> {
    if !auth.is_superposition_admin() {
        return Err(error_stack::report!(errors::ApiErrorResponse::Unauthorized));
    }

    ["context", "override", "change_reason"]
        .iter()
        .try_for_each(|&field| {
            if body.get(field).is_none() {
                Err(error_stack::report!(
                    errors::ApiErrorResponse::InvalidRequestData {
                        message: format!("missing required field: {field}"),
                    }
                ))
            } else {
                Ok(())
            }
        })?;

    let config = state.conf.superposition.get_inner();
    let response = SuperpositionClient::put_raw(config, "/context", body)
        .await
        .map_err(|e| map_superposition_err(e, "Failed to create context in Superposition"))?;

    Ok(services::ApplicationResponse::Json(response))
}
