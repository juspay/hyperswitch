//! Federated HyperSage Trace session minting.
//!
//! Proxies a session-mint request to HyperSage's
//! `POST /api/sage/session` on behalf of a Control Center user who
//! clicked the Trace launcher. The user's identity is read from the
//! verified `AuthToken`; HyperSage performs the authoritative
//! merchant-access gate against its own `auth_users.merchant_ids`
//! allowlist (see
//! <https://github.com/juspay/hypersage/blob/main/docs/SAGE_SESSION_API.md>).
//!
//! Mirrors the AI-service proxy pattern in
//! `crates/router/src/core/chat.rs` — outbound HTTP via
//! `external_services::http_client::send_request` against a shared
//! `state.conf.proxy` client, bearer-auth header pattern lifted from
//! `crates/router/src/utils/connector_onboarding/paypal.rs`.
//!
//! Tracking: juspay/hypersage#1040 (parent), #1066 (spike), #1067 (impl).
//!
//! # Open policy decisions for the dashboard / framework team
//!
//! Tagged with `// TODO(launch_trace#1040)` so reviewers can search
//! the file. Four spots:
//!
//! 1. Auth strategy — `DashboardNoPermissionAuth` vs `JWTAuth` with a
//!    specific `Permission`. See the PR description.
//! 2. Sage 401 → HS 502 inversion. Refusing to bubble 401 to CC so
//!    the dashboard doesn't log the user out on an HS-side infra bug.
//! 3. Role mapping `EntityType` → `{operator, viewer}` with the
//!    federated cap at `operator`. Tenant/Org → operator, Merchant/
//!    Profile → viewer, Internal → reject.
//! 4. 404 on flag-off — no oracle for "the env has federation
//!    pre-enabled".

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

/// Per-launch timeout (seconds). Sage mint is sub-second on the
/// happy path; the AI-service constant is the closest existing
/// timeout in the codebase. The `send_request` signature takes
/// `Option<u64>` (seconds), not `Duration`.
///
/// TODO(launch_trace#1040): promote a dedicated
/// `REQUEST_TIME_OUT_FOR_TRACE_FEDERATION` constant in
/// `crates/common_utils/src/consts.rs` once the dashboard/framework
/// review settles on a value. Reusing the AI-service constant for
/// now keeps the diff small.
const REQUEST_TIMEOUT_SECS: u64 = REQUEST_TIME_OUT_FOR_AI_SERVICE;

/// HyperSage's documented federated source tag for the audit chain.
const SAGE_SOURCE: &str = "hyperswitch-cc";

#[instrument(skip_all, fields(user_id, merchant_id))]
pub async fn launch_trace(
    state: SessionState,
    user_from_token: auth::UserFromToken,
) -> CustomResult<ApplicationResponse<launch_trace_api::LaunchTraceResponse>, LaunchTraceErrors> {
    let conf = state.conf.trace_integration.get_inner();

    // TODO(launch_trace#1040): no-oracle policy. Returning the same
    // `LaunchTraceErrors::FeatureDisabled` → 404 here as we'd return
    // for any other "this user can't federate" outcome (e.g., the
    // future role-rejection path). Confirms the design choice with
    // the dashboard team before flag-flip.
    if !conf.enabled {
        return Err(error_stack::Report::new(LaunchTraceErrors::FeatureDisabled))
            .attach_printable("Federated Trace flag is off in this env");
    }

    // Server-side derive the federated role from EntityType. NEVER
    // trust a role string from the request body (the request body is
    // empty by design — but defense in depth).
    //
    // TODO(launch_trace#1040): the EntityType → Sage-role mapping is
    // policy. Current choice:
    //   Tenant / Organization → "operator"
    //   Merchant / Profile    → "viewer"
    //   Internal              → reject (parallel to chat.rs:137-140)
    // matches the "federated cap at operator" policy in
    // hypersage/docs/SAGE_SESSION_API.md §2.
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
        // Internal users have other channels into HyperSage; they
        // should not be federating as a regular dashboard user.
        return Err(error_stack::Report::new(LaunchTraceErrors::Forbidden))
            .attach_printable("Internal roles are not eligible for federated Trace");
    }

    let sage_role = map_entity_type_to_sage_role(role_info.get_entity_type());

    // SAFETY: every id MUST come from the verified AuthToken, NEVER
    // from a request body. The route handler takes an empty body
    // (`()`) by design — there is nothing to spoof.
    let request_body = launch_trace_api::SageSessionRequest {
        user_id: user_from_token.user_id.clone(),
        merchant_id: user_from_token.merchant_id.get_string_repr().to_owned(),
        profile_id: user_from_token.profile_id.get_string_repr().to_owned(),
        org_id: user_from_token.org_id.get_string_repr().to_owned(),
        role: sage_role.to_owned(),
        source: SAGE_SOURCE.to_owned(),
        // Threaded once HS BE starts populating a per-AuthToken jti.
        // Optional during dark-launch per
        // hypersage/docs/SAGE_SESSION_API.md §2.
        subject_jti: None,
    };

    let url = format!("{}/api/sage/session", conf.sage_base_url);

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
            .attach_printable("Error when calling HyperSage /api/sage/session")?;

    // TODO(launch_trace#1040): error mapping policy. We deliberately
    // diverge from the upstream status in three places:
    //
    // - 401 from Sage → HS 502 (UpstreamCredentialsRejected). A 401
    //   here means HYPERSAGE_INFRA_KEY is wrong / rotated; the CC
    //   user's JWT was fine. Surfacing 401 to CC would log the user
    //   out on an HS-side infra bug.
    // - 404 from Sage → HS 404 (FeatureDisabled, same opaque message
    //   as flag-off). Preserves Sage's no-oracle policy at
    //   docs/SAGE_SESSION_API.md §3.
    // - 503 / network errors → HS 502-equivalent (UpstreamUnavailable).
    //
    // Confirm with dashboard team before flag-flip.
    let status = response.status();
    if !status.is_success() {
        if status.as_u16() == 401 {
            logger::error!(
                "HyperSage 401 — HYPERSAGE_INFRA_KEY rotation drift or misconfiguration"
            );
            return Err(error_stack::Report::new(
                LaunchTraceErrors::UpstreamCredentialsRejected,
            ));
        }
        if status.as_u16() == 404 {
            // Unknown user OR user-not-authorised-for-merchant on the
            // Sage side. Same opaque 404 as flag-off so the CC client
            // can't distinguish.
            return Err(error_stack::Report::new(LaunchTraceErrors::FeatureDisabled))
                .attach_printable("HyperSage returned 404 (user/merchant unauthorized)");
        }
        return Err(error_stack::Report::new(
            LaunchTraceErrors::UpstreamUnavailable,
        ))
        .attach_printable(format!("HyperSage returned status {status}"));
    }

    let parsed = response
        .json::<launch_trace_api::LaunchTraceResponse>()
        .await
        .change_context(LaunchTraceErrors::InternalServerError)
        .attach_printable("Failed to deserialize HyperSage handoff response")?;

    // Log only the host+path of the handoff URL — the `?t=<token>`
    // query is a bearer credential (60s TTL, single-use, but still
    // secret per docs/SAGE_SESSION_API.md §3). Without stripping it,
    // log aggregation would persist the token long enough for replay.
    if let Some(url_only) = parsed.handoff_url.split('?').next() {
        logger::info!(handoff_url_path = %url_only, "launch_trace: minted federated session");
    }

    Ok(ApplicationResponse::Json(parsed))
}

/// `EntityType` → Sage role string. Federated cap at `operator` per
/// `hypersage/docs/SAGE_SESSION_API.md` §2.
///
/// TODO(launch_trace#1040): confirm the mapping with the dashboard
/// team. Tenant + Organization both surface as `operator` because the
/// federated session is bound to a single merchant; broader-scoped
/// users still operate on one merchant at a time inside Trace.
fn map_entity_type_to_sage_role(entity: common_enums::EntityType) -> &'static str {
    use common_enums::EntityType;
    match entity {
        EntityType::Tenant | EntityType::Organization => "operator",
        EntityType::Merchant | EntityType::Profile => "viewer",
    }
}
