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
    payment_methods::{self as payment_methods_api, SectionError},
    payments as payment_types,
};
use common_utils::errors::ErrorSwitch;
use common_utils::{consts, id_type};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use crate::{
    core::{errors, payment_methods::client as pm_client, payments},
    routes::{app::ReqState, SessionState},
    services::{ApplicationResponse, AuthFlow},
    types::{api as api_types, domain},
};

/// Which integration the caller is building, taken from `X-Integration-Type`.
///
/// Defaults to `Client` when the header is absent or unrecognised, so an existing integration
/// that has never heard of the header keeps its current response shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationType {
    Client,
    Server,
}

impl IntegrationType {
    pub fn from_headers(headers: &actix_web::http::header::HeaderMap) -> Self {
        headers
            .get(consts::X_INTEGRATION_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| {
                if value.trim().eq_ignore_ascii_case("server") {
                    Self::Server
                } else {
                    Self::Client
                }
            })
            .unwrap_or(Self::Client)
    }

    pub fn is_server(self) -> bool {
        matches!(self, Self::Server)
    }
}

/// Wallets to mint session tokens for when the caller asks for the server shape.
///
/// The payments request carries no wallet list of its own, so the enrichment offers every wallet
/// we can mint a token for; the session core filters down to what the merchant actually has
/// enabled and eligible.
fn requested_wallets() -> Vec<api_models::enums::PaymentMethodType> {
    use api_models::enums::PaymentMethodType;
    vec![
        PaymentMethodType::ApplePay,
        PaymentMethodType::GooglePay,
        PaymentMethodType::Paypal,
        PaymentMethodType::SamsungPay,
    ]
}

/// Builds the error payload a degraded section carries, from the same error the standalone
/// endpoint would have surfaced.
fn section_error(error: &error_stack::Report<errors::ApiErrorResponse>) -> SectionError {
    // Route through the same conversion the HTTP layer uses, so an inline section error reads
    // identically to the body the standalone endpoint would have returned.
    let switched: api_models::errors::types::ApiErrorResponse = error.current_context().switch();
    let rendered = api_models::errors::types::ErrorResponse::from(&switched);
    SectionError {
        error: payment_methods_api::SectionErrorDetail {
            error_type: rendered.error_type.to_string(),
            message: rendered.message,
            code: rendered.code,
        },
    }
}

/// Attaches the payment-method list and wallet session tokens to a payments response.
///
/// Best-effort by design: the payment write has already committed by the time this runs, so a
/// failing section reports its own error inline and the response still succeeds. Turning a
/// section failure into a 5xx would hide a committed state change from the caller.
#[instrument(skip_all, fields(payment_id))]
pub async fn attach_server_context(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    response: &mut payment_types::PaymentsResponse,
) {
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    // Both reads observe the committed payment; neither reads the other's output.
    let (session_result, payment_methods_result) = futures::future::join(
        session_tokens(
            state,
            req_state,
            platform,
            profile_id,
            payment_id,
            header_payload,
        ),
        pm_client::list_payment_methods_client(
            state.clone(),
            platform.clone(),
            payment_id.clone(),
            // Merchant API key authenticated; there is no client secret to validate.
            None,
        ),
    )
    .await;

    response.session_tokens = Some(match session_result {
        Ok(session) => payment_types::SessionTokensResult::Success(Box::new(session)),
        Err(error) => {
            logger::warn!(?error, "server-integration: session tokens unavailable");
            payment_types::SessionTokensResult::Failed(section_error(&error))
        }
    });

    response.payment_method_list = Some(
        match payment_methods_result.and_then(|listing| json_body(listing, "payment_method_list")) {
            Ok(listing) => payment_methods_api::PaymentMethodListResult::Success(Box::new(listing)),
            Err(error) => {
                logger::warn!(
                    ?error,
                    "server-integration: payment-method list unavailable"
                );
                payment_methods_api::PaymentMethodListResult::Failed(section_error(&error))
            }
        },
    );
}

/// Runs the session-token core over every wallet we can mint for.
async fn session_tokens(
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
) -> errors::RouterResult<payment_types::PaymentsSessionResponse> {
    let response = Box::pin(payments::payments_core::<
        api_types::Session,
        payment_types::PaymentsSessionResponse,
        _,
        _,
        _,
        payments::PaymentData<api_types::Session>,
    >(
        state.clone(),
        req_state,
        platform.clone(),
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
        header_payload.clone(),
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
