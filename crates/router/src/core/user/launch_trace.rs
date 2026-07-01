use std::collections::HashMap;

use api_models::launch_trace as launch_trace_api;
use common_enums::EntityType;
use common_utils::{
    consts::REQUEST_TIME_OUT_FOR_AI_SERVICE,
    errors::CustomResult,
    request::{Method, RequestBuilder, RequestContent},
};
use diesel_models::enums::UserStatus;
use error_stack::ResultExt;
use external_services::http_client;
use hyperswitch_masking::PeekInterface;
use router_env::{instrument, logger, tracing};

use crate::{
    core::errors::launch_trace::LaunchTraceErrors,
    db::user_role::ListUserRolesByUserIdPayload,
    routes::SessionState,
    services::{authentication as auth, authorization::roles, ApplicationResponse},
};

// Reusing the AI-service timeout until a dedicated constant is added.
const REQUEST_TIMEOUT_SECS: u64 = REQUEST_TIME_OUT_FOR_AI_SERVICE;

const SAGE_SOURCE: &str = "hyperswitch-cc";

#[instrument(skip_all, fields(user_id, merchant_id))]
pub async fn launch_trace(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> CustomResult<ApplicationResponse<launch_trace_api::LaunchTraceResponse>, LaunchTraceErrors> {
    let conf = state.conf.sage.get_inner();

    if !conf.enabled {
        return Err(error_stack::Report::new(LaunchTraceErrors::SageDisabled))
            .attach_printable("sage flag is off in this env");
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
            .attach_printable("Internal roles are not eligible for sage");
    }

    let capped_role = map_entity_type_to_role(role_info.get_entity_type());

    // Send the full authorised scope on the mint request so sage can support
    // multi-merchant / multi-profile sessions. Shape mirrors
    // analytics::get_global_search_results (analytics.rs:3982).
    let scope = build_user_scope(&state, &user_from_token).await?;

    // Every id MUST come from the verified AuthToken — the route handler
    // takes an empty body by design.
    let request_body = launch_trace_api::SageSessionRequest {
        user_id: user_from_token.user_id.clone(),
        launch_merchant_id: user_from_token.merchant_id.get_string_repr().to_owned(),
        launch_profile_id: user_from_token.profile_id.get_string_repr().to_owned(),
        org_id: user_from_token.org_id.get_string_repr().to_owned(),
        role: capped_role.to_owned(),
        source: SAGE_SOURCE.to_owned(),
        subject_jti: None,
        scope,
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
            .change_context(LaunchTraceErrors::SageError)
            .attach_printable("Error calling sage")?;

    let status = response.status();
    if !status.is_success() {
        // 401 is a specific SRE-actionable signal (shared infra key drift).
        // Surface it loudly in logs; the caller still sees a generic 5xx.
        if status.as_u16() == 401 {
            logger::error!("sage: upstream 401 — likely infra key drift");
        }
        return Err(error_stack::Report::new(LaunchTraceErrors::SageError))
            .attach_printable(format!("sage returned status {status}"));
    }

    let parsed = response
        .json::<launch_trace_api::LaunchTraceResponse>()
        .await
        .change_context(LaunchTraceErrors::SageError)
        .attach_printable("Failed to deserialize sage handoff response")?;

    // Strip the `?t=…` bearer before logging — it's a single-use credential
    // that must not survive log aggregation.
    if let Some(url_only) = parsed.handoff_url.split('?').next() {
        logger::info!(handoff_url_path = %url_only, "sage: minted federated session");
    }

    Ok(ApplicationResponse::Json(parsed))
}

/// Enumerate the caller's active user_roles in the JWT's org and emit one
/// generic `ScopeEntry` per row. Wildcards are implicit at each level — a
/// merchant-level entry covers all profiles under that merchant without any
/// explicit wildcard field.
async fn build_user_scope(
    state: &SessionState,
    user_from_token: &auth::UserFromToken,
) -> CustomResult<Vec<launch_trace_api::ScopeEntry>, LaunchTraceErrors> {
    let tenant_id = user_from_token
        .tenant_id
        .as_ref()
        .unwrap_or(&state.tenant.tenant_id);

    let user_roles = state
        .global_store
        .list_user_roles_by_user_id(ListUserRolesByUserIdPayload {
            user_id: &user_from_token.user_id,
            tenant_id,
            org_id: Some(&user_from_token.org_id),
            merchant_id: None,
            profile_id: None,
            entity_id: None,
            version: None,
            status: Some(UserStatus::Active),
            limit: None,
        })
        .await
        .change_context(LaunchTraceErrors::InternalServerError)
        .attach_printable("Failed to list user roles for scope enumeration")?;

    let mut scope: Vec<launch_trace_api::ScopeEntry> = Vec::with_capacity(user_roles.len());

    for user_role in &user_roles {
        let Some((entity_id, entity_type)) = user_role.get_entity_id_and_type() else {
            continue;
        };
        scope.push(user_role_to_scope_entry(user_role, entity_id, entity_type));
    }

    // Deterministic ordering for clean wire diffs across identical calls.
    scope.sort_by(|a, b| {
        a.entity_type
            .cmp(&b.entity_type)
            .then_with(|| a.entity_id.cmp(&b.entity_id))
    });

    Ok(scope)
}

fn user_role_to_scope_entry(
    user_role: &diesel_models::user_role::UserRole,
    entity_id: String,
    entity_type: EntityType,
) -> launch_trace_api::ScopeEntry {
    let mut path: HashMap<String, String> = HashMap::new();
    path.insert(
        "tenant_id".to_owned(),
        user_role.tenant_id.get_string_repr().to_owned(),
    );
    let (type_name, include_org, include_merchant) = match entity_type {
        EntityType::Tenant => ("tenant", false, false),
        EntityType::Organization => ("organization", false, false),
        EntityType::Merchant => ("merchant", true, false),
        EntityType::Profile => ("profile", true, true),
    };
    if include_org {
        if let Some(org_id) = user_role.org_id.as_ref() {
            path.insert("org_id".to_owned(), org_id.get_string_repr().to_owned());
        }
    }
    if include_merchant {
        if let Some(merchant_id) = user_role.merchant_id.as_ref() {
            path.insert(
                "merchant_id".to_owned(),
                merchant_id.get_string_repr().to_owned(),
            );
        }
    }
    launch_trace_api::ScopeEntry {
        entity_type: type_name.to_owned(),
        entity_id,
        path,
        constraints: HashMap::new(),
    }
}

fn map_entity_type_to_role(entity: EntityType) -> &'static str {
    match entity {
        EntityType::Tenant | EntityType::Organization => "operator",
        EntityType::Merchant | EntityType::Profile => "viewer",
    }
}
