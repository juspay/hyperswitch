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
use common_utils::{consts, errors::ErrorSwitch, id_type};
use error_stack::ResultExt;
use router_env::{instrument, logger, tracing};

use crate::{
    core::{errors, payment_methods::client as pm_client, payments},
    routes::{app::ReqState, SessionState},
    services::{ApplicationResponse, AuthFlow},
    types::{api as api_types, domain},
};

/// How long a single section may take before it is reported as degraded.
///
/// The session core runs with `CallConnectorAction::Trigger`, so it can make outbound connector
/// calls, and the payment-method list is served by the modular service over HTTP. The payment has
/// already committed by the time either runs, so a section that hangs would hold a response the
/// caller is entitled to. Each section is bounded independently so a slow one cannot starve the
/// other.
const SECTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

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
            .map(|value| match value.trim() {
                value if value.eq_ignore_ascii_case("server") => Self::Server,
                value if value.eq_ignore_ascii_case("client") => Self::Client,
                // Falling back to `Client` keeps a malformed header from failing the payment, but
                // a caller that meant `server` would otherwise just never see the extra sections.
                unrecognised => {
                    logger::warn!(
                        header = consts::X_INTEGRATION_TYPE,
                        value = unrecognised,
                        "unrecognised integration type, defaulting to client"
                    );
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
    state: &SessionState,
    req_state: ReqState,
    platform: &domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: &id_type::PaymentId,
    header_payload: &hyperswitch_domain_models::payments::HeaderPayload,
    response: &mut payment_types::PaymentsResponse,
) {
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    // Both reads observe the committed payment; neither reads the other's output. Each is
    // bounded separately so one slow section still lets the other through.
    let (session_result, payment_methods_result) = Box::pin(futures::future::join(
        tokio::time::timeout(
            SECTION_TIMEOUT,
            Box::pin(session_tokens(
                state,
                req_state,
                platform,
                profile_id,
                payment_id,
                header_payload,
            )),
        ),
        tokio::time::timeout(
            SECTION_TIMEOUT,
            pm_client::list_payment_methods_client(
                state.clone(),
                platform.clone(),
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

#[cfg(test)]
mod tests {
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};

    use super::*;

    fn headers(value: Option<&str>) -> HeaderMap {
        let mut map = HeaderMap::new();
        value.into_iter().for_each(|value| {
            map.insert(
                HeaderName::from_static(consts::X_INTEGRATION_TYPE),
                HeaderValue::from_str(value).expect("header value"),
            );
        });
        map
    }

    #[test]
    fn absent_header_is_client() {
        assert_eq!(
            IntegrationType::from_headers(&headers(None)),
            IntegrationType::Client
        );
    }

    #[test]
    fn server_opts_in() {
        assert_eq!(
            IntegrationType::from_headers(&headers(Some("server"))),
            IntegrationType::Server
        );
    }

    #[test]
    fn server_is_case_and_whitespace_insensitive() {
        ["SERVER", "Server", "  server  "]
            .into_iter()
            .for_each(|value| {
                assert_eq!(
                    IntegrationType::from_headers(&headers(Some(value))),
                    IntegrationType::Server,
                    "{value:?} should opt in"
                );
            });
    }

    #[test]
    fn client_and_unrecognised_values_stay_client() {
        // A typo must not fail the payment; it degrades to the existing response shape.
        ["client", "CLIENT", "sever", "banana", ""]
            .into_iter()
            .for_each(|value| {
                assert_eq!(
                    IntegrationType::from_headers(&headers(Some(value))),
                    IntegrationType::Client,
                    "{value:?} should not opt in"
                );
            });
    }

    #[test]
    fn only_server_is_server() {
        assert!(IntegrationType::Server.is_server());
        assert!(!IntegrationType::Client.is_server());
    }

    #[test]
    fn every_eligible_wallet_is_requested() {
        // Empty means "all eligible" to the session core; a non-empty list would silently
        // exclude any type not named in it.
        assert!(requested_wallets().is_empty());
    }
}
