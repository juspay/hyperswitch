//! Server-integration enrichment for the create-intent response.
//!
//! A caller that sends `X-Integration-Type: server` on `POST /payments` gets the payment
//! response it always got, plus the combined payment-method list and the wallet session tokens —
//! everything a server-driven checkout needs to render, in one round trip. Client integrations,
//! and callers that send no header at all, are unaffected.
//!
//! The fetch itself is shared with update intent — see [`super::server_integration`].

use api_models::payments as payment_types;
use common_utils::id_type;
use hyperswitch_domain_models::payments::HeaderPayload;
use router_env::{instrument, tracing};

pub use super::server_integration::integration_type_from_headers;
use super::server_integration::{fetch_server_context, ServerContext};
use crate::{
    routes::{app::ReqState, SessionState},
    types::domain,
};

/// Attaches the payment-method list and wallet session tokens to a create-intent response.
///
/// Best-effort by design: the intent has already been created by the time this runs, so a failing
/// section reports its own error inline and the response still succeeds — the caller must still
/// learn the `payment_id` and `client_secret` of the intent it now owns.
#[instrument(skip_all)]
pub async fn attach_server_context(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: HeaderPayload,
    response: &mut payment_types::PaymentsResponse,
) {
    let ServerContext {
        session_tokens,
        payment_method_list,
    } = fetch_server_context(
        state,
        req_state,
        platform,
        profile_id,
        payment_id,
        header_payload,
    )
    .await;

    response.session_tokens = Some(session_tokens);
    response.payment_method_list = Some(payment_method_list);
}
