use api_models::launch_trace as launch_trace_api;
use common_utils::{
    consts::REQUEST_TIME_OUT_FOR_AI_SERVICE,
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use external_services::http_client;
use hyperswitch_masking::PeekInterface;
use router_env::{instrument, logger, tracing};

use crate::{
    core::errors::launch_trace::LaunchTraceErrors,
    routes::SessionState,
    services::{authentication as auth, authorization::roles, ApplicationResponse},
};

// Reusing the AI-service timeout until a dedicated constant is added.
const REQUEST_TIMEOUT_SECS: u64 = REQUEST_TIME_OUT_FOR_AI_SERVICE;

const INTEGRATION_SOURCE: &str = "hyperswitch-cc";

#[instrument(skip_all, fields(user_id, merchant_id))]
pub async fn launch_trace(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> CustomResult<ApplicationResponse<launch_trace_api::LaunchTraceResponse>, LaunchTraceErrors> {
    let conf = state.conf.trace_integration.get_inner();

    if !conf.enabled {
        return Err(error_stack::Report::new(LaunchTraceErrors::FeatureDisabled))
            .attach_printable("Federated Trace flag is off in this env");
    }

    let role_info = roles::RoleInfo::from_role_id_org_id_tenant_id(
        &state,
        &user_from_token.role_id,
        &user_from_token.org_id,
        user_from_token
            .tenant_id
            .as_ref()
            .unwrap_or(&state.tenant.tenant_id),
    )
    .await
    .change_context(LaunchTraceErrors::InternalServerError)
    .attach_printable("Failed to retrieve role information")?;

    if role_info.is_internal() {
        return Err(error_stack::Report::new(LaunchTraceErrors::Forbidden))
            .attach_printable("Internal roles are not eligible for federated Trace");
    }

    let capped_role = map_entity_type_to_role(role_info.get_entity_type());

    // Every id MUST come from the verified AuthToken — the route handler
    // takes an empty body by design.
    let request_body = launch_trace_api::MintSessionRequest {
        user_id: user_from_token.user_id.clone(),
        merchant_id: user_from_token.merchant_id.get_string_repr().to_owned(),
        profile_id: user_from_token.profile_id.get_string_repr().to_owned(),
        org_id: user_from_token.org_id.get_string_repr().to_owned(),
        role: capped_role.to_owned(),
        source: INTEGRATION_SOURCE.to_owned(),
        subject_jti: None,
    };

    let url = format!("{}{}", conf.base_url, conf.mint_path);

    let request = RequestBuilder::new()
        .method(Method::Post)
        .url(&url)
        .attach_default_headers()
        .header(
            "Authorization",
            &format!("Bearer {}", conf.infra_key.peek()),
        )
        .set_body(RequestContent::Json(Box::new(request_body)))
        .build();

    let response =
        http_client::send_request(&state.conf.proxy, request, Some(REQUEST_TIMEOUT_SECS))
            .await
            .change_context(LaunchTraceErrors::UpstreamUnavailable)
            .attach_printable("Error calling trace integration upstream")?;

    let status = response.status();
    if !status.is_success() {
        // 401 maps to 502-shaped error — the dashboard JWT was valid;
        // the failure is an infra-key drift on our side. Refusing to
        // bubble 401 prevents an upstream misconfig from logging the
        // CC user out.
        if status.as_u16() == 401 {
            logger::error!("trace integration: upstream returned 401 — infra key drift");
            return Err(error_stack::Report::new(
                LaunchTraceErrors::UpstreamCredentialsRejected,
            ));
        }
        // 404 stays 404 with the same opaque message as flag-off, so
        // the caller cannot distinguish "feature off" from "user/merchant
        // not authorised upstream".
        if status.as_u16() == 404 {
            return Err(error_stack::Report::new(LaunchTraceErrors::FeatureDisabled))
                .attach_printable("Upstream returned 404 (user/merchant unauthorized)");
        }
        return Err(error_stack::Report::new(
            LaunchTraceErrors::UpstreamUnavailable,
        ))
        .attach_printable(format!("Upstream returned status {status}"));
    }

    let parsed = response
        .json::<launch_trace_api::LaunchTraceResponse>()
        .await
        .change_context(LaunchTraceErrors::InternalServerError)
        .attach_printable("Failed to deserialize handoff response")?;

    // Strip the `?t=…` bearer before logging — it's a single-use credential
    // that must not survive log aggregation.
    if let Some(url_only) = parsed.handoff_url.split('?').next() {
        logger::info!(handoff_url_path = %url_only, "launch_trace: minted federated session");
    }

    Ok(ApplicationResponse::Json(parsed))
}

fn map_entity_type_to_role(entity: common_enums::EntityType) -> &'static str {
    use common_enums::EntityType;
    match entity {
        EntityType::Tenant | EntityType::Organization => "operator",
        EntityType::Merchant | EntityType::Profile => "viewer",
    }
}
