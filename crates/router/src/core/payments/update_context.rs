//! Server-integration enrichment for payments responses.
//!
//! A caller that sends `X-Integration-Type: server` gets the payment response it always got,
//! plus the two artifacts its checkout would otherwise fetch in separate calls: the combined
//! payment-method list and the wallet session tokens. Client integrations, and callers that
//! send no header at all, are unaffected.
//!
//! This module adds no business logic. It calls the two existing cores — the same ones behind
//! `POST /payments/session_tokens` and `GET /payments/{id}/client` — and attaches their results
//! to the response. Both reads run after the write they depend on, and concurrently with each
//! other, since neither reads the other's output.

use api_models::{
    payment_methods as payment_methods_api,
    payments::{self as payment_types, IntegrationType},
};
use common_utils::{consts, errors::ErrorSwitch, id_type};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use crate::{
    core::{configs::dimension_state, errors, payment_methods::client as pm_client, payments},
    routes::{app::ReqState, SessionState},
    services::{ApplicationResponse, AuthFlow},
    types::{api as api_types, domain},
};

/// How long a single section may run before it is reported as degraded.
///
/// The same bound an outgoing connector call gets. The session core makes outbound calls of its
/// own — the Apple Pay session call among them — and those already run under
/// [`consts::REQUEST_TIME_OUT`], so a tighter bound here would cut short a call the standalone
/// endpoint would have let finish. The payment has already committed by the time either section
/// runs, so a section that hangs would hold back a response the caller is entitled to; each is
/// bounded separately so a slow one cannot starve the other.
const SECTION_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(consts::REQUEST_TIME_OUT);

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

    // Tested against the accepted spellings directly rather than inferred from `integration_type`
    // being `Client`: that inference only holds while `Client` is the fallback, and would stop
    // reporting anything the day the fallback changed.
    let unrecognised = value.filter(|value| {
        let value = value.trim();
        !value.eq_ignore_ascii_case("server") && !value.eq_ignore_ascii_case("client")
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

/// Resolves the integration type the merchant is configured for: Superposition's
/// `payments.integration_type`, keyed on the processor merchant, with the `configs` table as
/// fallback and `client_and_server` when neither has a value.
pub async fn merchant_integration_type(
    state: &SessionState,
    platform: &domain::Platform,
) -> common_enums::MerchantIntegrationType {
    dimension_state::Dimensions::new()
        .with_processor_merchant_id(platform.get_processor().get_processor_merchant_id())
        .with_provider_merchant_id(platform.get_provider().get_provider_merchant_id())
        .get_merchant_integration_type(
            state.store.as_ref(),
            state.superposition_service.as_ref(),
            None,
        )
        .await
}

/// Rejects a request whose `X-Integration-Type` header does not match the integration the
/// merchant is configured for.
///
/// A `client_and_server` merchant may send either value. A `client` or `server` merchant must
/// send its own; an absent header reads as `client`, so a `server` merchant has to send the
/// header on every request this check guards.
pub fn validate_integration_type(
    header: IntegrationType,
    merchant: common_enums::MerchantIntegrationType,
) -> errors::RouterResult<()> {
    let allowed = match merchant {
        common_enums::MerchantIntegrationType::ClientAndServer => true,
        common_enums::MerchantIntegrationType::Client => !header.is_server(),
        common_enums::MerchantIntegrationType::Server => header.is_server(),
    };

    common_utils::fp_utils::when(!allowed, || {
        Err(error_stack::report!(
            errors::ApiErrorResponse::InvalidRequestData {
                message: format!(
                    "`{}` header value `{}` does not match the merchant integration type `{merchant}`",
                    consts::X_INTEGRATION_TYPE,
                    header.as_header_value()
                ),
            }
        ))
    })
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

/// Attaches the payment-method list and wallet session tokens to a payments response.
///
/// Best-effort by design: the payment write has already committed by the time this runs, so a
/// failing section reports its own error inline and the response still succeeds. Turning a
/// section failure into a 5xx would hide a committed state change from the caller.
#[instrument(skip_all, fields(payment_id))]
pub async fn attach_server_context(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
    response: &mut payment_types::PaymentsResponse,
) {
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    // Logged unconditionally on entry. This function runs only for a caller that opted in with
    // `X-Integration-Type: server`, so the presence of this line is what separates "the header
    // arrived and enrichment ran" from "the request never reached this path" — the two cases a
    // response missing both sections cannot otherwise distinguish.
    logger::info!(
        timeout_secs = SECTION_TIMEOUT.as_secs(),
        "server-integration: enriching payments response with session tokens and payment-method list"
    );

    let started = std::time::Instant::now();

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
            Box::pin(payment_method_list(state, platform, payment_id)),
        ),
    ))
    .await;

    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    let session_result = session_result.unwrap_or_else(|_| Err(timed_out("session_tokens")));
    let payment_methods_result =
        payment_methods_result.unwrap_or_else(|_| Err(timed_out("payment_method_list")));

    response.session_tokens = Some(match session_result {
        Ok(session) => {
            logger::info!(
                session_token_count = session.session_token.len(),
                "server-integration: session tokens attached"
            );
            payment_types::SessionTokensResult::Success(Box::new(session))
        }
        Err(error) => {
            logger::warn!(?error, "server-integration: session tokens unavailable");
            payment_types::SessionTokensResult::Failed {
                error: section_error(&error),
            }
        }
    });

    response.payment_method_list = Some(match payment_methods_result {
        Ok(listing) => {
            logger::info!(
                payment_methods_enabled_count = listing.payment_methods_enabled.len(),
                customer_payment_methods_count = listing.customer_payment_methods.len(),
                "server-integration: payment-method list attached"
            );
            payment_methods_api::PaymentMethodListResult::Success(Box::new(listing))
        }
        Err(error) => {
            logger::warn!(
                ?error,
                "server-integration: payment-method list unavailable"
            );
            payment_methods_api::PaymentMethodListResult::Failed {
                error: section_error(&error),
            }
        }
    });

    // Reports what each section actually produced. A blanket "complete" here would read as
    // success on a response whose sections both carry errors.
    logger::info!(
        elapsed_ms,
        session_tokens_ok = matches!(
            response.session_tokens,
            Some(payment_types::SessionTokensResult::Success(_))
        ),
        payment_method_list_ok = matches!(
            response.payment_method_list,
            Some(payment_methods_api::PaymentMethodListResult::Success(_))
        ),
        "server-integration: enrichment complete"
    );
}

/// Runs the session-token core for this payment.
async fn session_tokens(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> errors::RouterResult<payment_types::PaymentsSessionResponse> {
    logger::info!("server-integration: calling session-token core");

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
            wallets: Vec::new(),
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

/// Runs the payment-method-list core the client endpoint is backed by.
///
/// A thin wrapper over the core call so this section announces itself before it starts, the same
/// way [`session_tokens`] does — without it, a section that hangs until [`SECTION_TIMEOUT`] leaves
/// no trace of having been entered.
async fn payment_method_list(
    state: SessionState,
    platform: domain::Platform,
    payment_id: &id_type::PaymentId,
) -> errors::RouterResult<payment_methods_api::ClientPaymentMethodsListResponse> {
    logger::info!("server-integration: calling payment-method-list core");

    let response = Box::pin(pm_client::list_payment_methods_client(
        state,
        platform,
        payment_id.clone(),
        // Merchant API key authenticated; there is no client secret to validate.
        None,
    ))
    .await?;

    json_body(response, "payment_method_list")
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
