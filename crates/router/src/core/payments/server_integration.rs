//! Shared plumbing for the server-integration (`X-Integration-Type: server`) response shape.
//!
//! Both the create-intent and update-intent flows hand the caller the two artifacts its checkout
//! would otherwise fetch in separate calls: the combined payment-method list and the wallet
//! session tokens. This module owns the header parsing and the concurrent fetch of those two
//! sections; the per-flow modules ([`super::create_intent`], [`super::update_intent`]) decide
//! when to run it and attach the result to their response.
//!
//! No business logic lives here. It calls the two existing cores — the same ones behind
//! `POST /payments/session_tokens` and `GET /payments/{id}/client` — and reports each outcome.
//! Both reads run after the write they depend on, and concurrently with each other, since neither
//! reads the other's output.

use api_models::{
    payment_methods as payment_methods_api,
    payments::{self as payment_types, IntegrationType},
};
use common_utils::{consts, errors::ErrorSwitch, id_type};
use error_stack::ResultExt;
use hyperswitch_domain_models::payments::HeaderPayload;
use router_env::{instrument, logger, tracing};

use crate::{
    consts::SERVER_INTEGRATION_SECTION_TIMEOUT as SECTION_TIMEOUT,
    core::{errors, payment_methods::client as pm_client, payments},
    routes::{app::ReqState, SessionState},
    services::{ApplicationResponse, AuthFlow},
    types::{api as api_types, domain},
};

/// Reads [`IntegrationType`] from the request headers.
///
/// Falling back to `Client` keeps a malformed header from failing the payment, but a caller that
/// meant `server` would otherwise silently never see the extra sections — so that case is logged.
pub fn integration_type_from_headers(
    headers: &actix_web::http::header::HeaderMap,
) -> IntegrationType {
    let value = headers
        .get(consts::X_INTEGRATION_TYPE)
        .and_then(|value| value.to_str().ok());

    let integration_type = IntegrationType::from_header_value(value);

    let unrecognised = value.filter(|value| {
        integration_type == IntegrationType::Client && !value.trim().eq_ignore_ascii_case("client")
    });

    if let Some(value) = unrecognised {
        logger::warn!(
            header = consts::X_INTEGRATION_TYPE,
            value,
            "unrecognised integration type, defaulting to client"
        );
    }

    integration_type
}

/// Wallets to mint session tokens for when the caller asks for the server shape.
///
/// Empty on purpose. The session core reads an empty list as "every eligible type": it keeps a
/// payment method type when `requested_payment_method_types.contains(..) || ..is_empty()`, so an
/// empty list yields a token for every `InvokeSdkClient`-capable method the merchant has enabled.
/// Naming wallets explicitly here would silently exclude anything not on the list — Klarna SDK
/// sessions today, and every type added later.
fn requested_wallets() -> Vec<api_models::enums::PaymentMethodType> {
    Vec::new()
}

/// Builds the error payload a degraded section carries, from the same error the standalone
/// endpoint would have surfaced.
fn section_error(
    error: &error_stack::Report<errors::ApiErrorResponse>,
) -> Box<api_models::errors::types::ErrorResponse> {
    // Route through the same conversion the HTTP layer uses, so an inline section error reads
    // identically to the body the standalone endpoint would have returned.
    let switched: api_models::errors::types::ApiErrorResponse = error.current_context().switch();
    Box::new(api_models::errors::types::ErrorResponse::from(&switched))
}

/// The error a section reports when it exceeds [`SECTION_TIMEOUT`].
fn timed_out(section: &str) -> error_stack::Report<errors::ApiErrorResponse> {
    error_stack::report!(errors::ApiErrorResponse::InternalServerError).attach_printable(format!(
        "server-integration: {section} exceeded {}s and was reported as degraded",
        SECTION_TIMEOUT.as_secs()
    ))
}

/// The two server-integration sections, each already in the shape the response carries.
pub struct ServerContext {
    pub session_tokens: payment_types::SessionTokensResult,
    pub payment_method_list: payment_methods_api::PaymentMethodListResult,
}

/// Fetches the wallet session tokens and the combined payment-method list for a committed
/// payment, concurrently.
///
/// Best-effort by design: the payment write has already committed by the time this runs, so a
/// failing section reports its own error inline rather than failing the whole response. Turning a
/// section failure into a 5xx would hide a committed state change from the caller. That is why
/// this is a `join` and not a `try_join`: one section's failure must not cancel the other.
#[instrument(skip_all, fields(payment_id))]
pub async fn fetch_server_context(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: HeaderPayload,
) -> ServerContext {
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    // Both reads observe the committed payment; neither reads the other's output. Each is
    // bounded separately so one slow section still lets the other through.
    let (session_result, payment_methods_result) = Box::pin(futures::future::join(
        tokio::time::timeout(
            SECTION_TIMEOUT,
            Box::pin(session_tokens(
                state.clone(),
                req_state,
                platform.clone(),
                profile_id,
                payment_id,
                header_payload,
            )),
        ),
        tokio::time::timeout(
            SECTION_TIMEOUT,
            pm_client::list_payment_methods_client(
                state,
                platform,
                payment_id.clone(),
                // Merchant API key authenticated; there is no client secret to validate.
                None,
            ),
        ),
    ))
    .await;

    let session_result = session_result.unwrap_or_else(|_| Err(timed_out("session_tokens")));
    let payment_methods_result =
        payment_methods_result.unwrap_or_else(|_| Err(timed_out("payment_method_list")));

    let session_tokens = match session_result {
        Ok(session) => payment_types::SessionTokensResult::Success(Box::new(session)),
        Err(error) => {
            logger::warn!(?error, "server-integration: session tokens unavailable");
            payment_types::SessionTokensResult::Failed {
                error: section_error(&error),
            }
        }
    };

    let payment_method_list = match payment_methods_result
        .and_then(|listing| json_body(listing, "payment_method_list"))
    {
        Ok(listing) => payment_methods_api::PaymentMethodListResult::Success(Box::new(listing)),
        Err(error) => {
            logger::warn!(
                ?error,
                "server-integration: payment-method list unavailable"
            );
            payment_methods_api::PaymentMethodListResult::Failed {
                error: section_error(&error),
            }
        }
    };

    ServerContext {
        session_tokens,
        payment_method_list,
    }
}

/// Runs the session-token core over every wallet we can mint for.
async fn session_tokens(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: HeaderPayload,
) -> errors::RouterResult<payment_types::PaymentsSessionResponse> {
    let response = Box::pin(payments::payments_core::<
        api_types::Session,
        payment_types::PaymentsSessionResponse,
        _,
        _,
        _,
        payments::PaymentData<api_types::Session>,
    >(
        state,
        req_state,
        platform,
        profile_id,
        payments::PaymentSession,
        payment_types::PaymentsSessionRequest {
            payment_id: payment_id.clone(),
            client_secret: None,
            wallets: requested_wallets(),
            merchant_connector_details: None,
        },
        AuthFlow::Merchant,
        payments::CallConnectorAction::Trigger,
        None,
        None,
        header_payload,
        None,
    ))
    .await?;

    // Returned whole: the caller gets `session_token` and `vault_details` (the internal vault
    // SDK authorization) exactly as the standalone endpoint would have returned them.
    json_body(response, "session_tokens")
}

/// A core response can only contribute when it is a plain JSON body.
fn json_body<T>(response: ApplicationResponse<T>, section: &str) -> errors::RouterResult<T> {
    match response {
        ApplicationResponse::Json(payload) | ApplicationResponse::JsonWithHeaders((payload, _)) => {
            Ok(payload)
        }
        _ => Err(error_stack::report!(
            errors::ApiErrorResponse::InternalServerError
        ))
        .attach_printable_lazy(|| {
            format!("server-integration: {section} core returned a non-JSON response")
        }),
    }
}
