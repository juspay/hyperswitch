use std::collections::{HashMap, HashSet};

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

    // Enumerate every active role the user holds in the JWT's org so the
    // mint request carries the full authorised scope, not just the current
    // view. Upstream intersects this with its own allowlist as defence in
    // depth (identical shape to analytics::get_global_search_results at
    // crates/router/src/analytics.rs:3982).
    let scope = build_user_scope(&state, &user_from_token).await?;

    // Every id MUST come from the verified AuthToken — the route handler
    // takes an empty body by design.
    let request_body = launch_trace_api::MintSessionRequest {
        user_id: user_from_token.user_id.clone(),
        launch_merchant_id: user_from_token.merchant_id.get_string_repr().to_owned(),
        launch_profile_id: user_from_token.profile_id.get_string_repr().to_owned(),
        org_id: user_from_token.org_id.get_string_repr().to_owned(),
        role: capped_role.to_owned(),
        source: INTEGRATION_SOURCE.to_owned(),
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

/// Enumerate the caller's active user_roles in the JWT's org, transform each
/// row per its `entity_type`, and collapse duplicates so each merchant
/// appears at most once in the returned Vec. Merchant-level rows produce a
/// wildcard entry (all profiles); Profile-level rows collapse per-merchant
/// into a concrete profile list. Tenant / Organization rows expand to a
/// wildcard entry for every merchant in the org.
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

    // Wildcard-merchant set shadows any per-profile lists for the same
    // merchant (a merchant-level grant subsumes any profile-level grant).
    let mut wildcard_merchants: HashSet<String> = HashSet::new();
    let mut profile_lists: HashMap<String, Vec<String>> = HashMap::new();
    let mut expand_org_to_all_merchants = false;

    for user_role in user_roles {
        let Some((entity_id, entity_type)) = user_role.get_entity_id_and_type() else {
            continue;
        };

        match entity_type {
            EntityType::Tenant | EntityType::Organization => {
                expand_org_to_all_merchants = true;
            }
            EntityType::Merchant => {
                wildcard_merchants.insert(entity_id);
            }
            EntityType::Profile => {
                if let Some(merchant_id) = user_role.merchant_id.as_ref() {
                    profile_lists
                        .entry(merchant_id.get_string_repr().to_owned())
                        .or_default()
                        .push(entity_id);
                }
            }
        }
    }

    // Any Tenant / Organization role reachable from the org grants wildcard
    // access to every merchant under it.
    if expand_org_to_all_merchants {
        let merchants = state
            .store
            .list_merchant_accounts_by_organization_id(&user_from_token.org_id)
            .await
            .change_context(LaunchTraceErrors::InternalServerError)
            .attach_printable("Failed to list merchant accounts for org-level scope expansion")?;
        for merchant in merchants {
            wildcard_merchants.insert(merchant.get_id().get_string_repr().to_owned());
        }
    }

    let mut scope: Vec<launch_trace_api::ScopeEntry> =
        Vec::with_capacity(wildcard_merchants.len() + profile_lists.len());

    for merchant_id in &wildcard_merchants {
        scope.push(launch_trace_api::ScopeEntry {
            merchant_id: merchant_id.clone(),
            profile_ids: launch_trace_api::ScopeProfiles::All,
        });
    }
    for (merchant_id, mut profiles) in profile_lists {
        if wildcard_merchants.contains(&merchant_id) {
            continue;
        }
        profiles.sort();
        profiles.dedup();
        scope.push(launch_trace_api::ScopeEntry {
            merchant_id,
            profile_ids: launch_trace_api::ScopeProfiles::Some(profiles),
        });
    }
    // Deterministic ordering so the wire body diffs cleanly across calls.
    scope.sort_by(|a, b| a.merchant_id.cmp(&b.merchant_id));

    Ok(scope)
}

fn map_entity_type_to_role(entity: EntityType) -> &'static str {
    match entity {
        EntityType::Tenant | EntityType::Organization => "operator",
        EntityType::Merchant | EntityType::Profile => "viewer",
    }
}
