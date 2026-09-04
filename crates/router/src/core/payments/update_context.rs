//! Composite server-to-server update: apply an intent update, then return the artifacts the
//! client SDK needs to carry on with the new amount.
//!
//! This module adds no business logic. It calls three cores that already exist and joins their
//! results, so the composite response can never drift from the standalone endpoints:
//!
//! 1. the update core, exactly as `POST /payments/{id}` invokes it;
//! 2. the session-token core, as `POST /payments/session_tokens` invokes it;
//! 3. the combined payment-method listing, as `GET /payments/{id}/client` invokes it.
//!
//! Step 1 is a hard gate. Steps 2 and 3 read the intent, so they must observe the committed
//! amount and cannot race the update — but they do not read each other, so they run concurrently.

use api_models::payments as payment_types;
use common_utils::{fp_utils, id_type};
use error_stack::report;
use router_env::{instrument, logger, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse},
        payment_methods::client as pm_client,
        payments,
    },
    routes::{app::ReqState, SessionState},
    services::{ApplicationResponse, AuthFlow},
    types::{api as api_types, domain},
};

/// Codes surfaced in `warnings`. Each maps to the standalone endpoint a caller can re-invoke to
/// fill the gap, so a degraded response is recoverable without repeating the update.
const SESSION_TOKENS_FAILED: &str = "SESSION_TOKENS_FAILED";
const PAYMENT_METHODS_FAILED: &str = "PAYMENT_METHODS_FAILED";

/// Rejects a request that asks for no change, or an impossible one, before anything is executed.
fn validate_request(req: &payment_types::PaymentsUpdateContextRequest) -> errors::RouterResult<()> {
    fp_utils::when(req.amount.is_none() && req.currency.is_none(), || {
        Err(report!(errors::ApiErrorResponse::InvalidRequestData {
            message: "at least one of `amount` or `currency` must be provided".to_string(),
        }))
    })?;

    fp_utils::when(
        req.amount
            .is_some_and(|amount| amount.get_amount_as_i64() <= 0),
        || {
            Err(report!(errors::ApiErrorResponse::InvalidRequestData {
                message: "`amount` must be greater than zero".to_string(),
            }))
        },
    )
}

/// Turns the composite request into the request the update core already understands.
fn to_payments_request(
    req: &payment_types::PaymentsUpdateContextRequest,
    payment_id: &id_type::PaymentId,
) -> payment_types::PaymentsRequest {
    payment_types::PaymentsRequest {
        payment_id: Some(payment_types::PaymentIdType::PaymentIntentId(
            payment_id.clone(),
        )),
        amount: req.amount.map(payment_types::Amount::from),
        currency: req.currency,
        ..Default::default()
    }
}

/// Unwraps a section result, recording a warning instead of failing the request when it errored.
///
/// The update has already committed by the time this runs, so turning a section failure into a
/// 5xx would hide a real state change from the caller. That is the one outcome this endpoint
/// refuses to produce.
fn degrade_on_error<T, E: std::fmt::Debug>(
    result: Result<T, E>,
    section: payment_types::UpdateContextSection,
    code: &str,
    warnings: &mut Vec<payment_types::UpdateContextWarning>,
) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            logger::warn!(
                ?error,
                ?section,
                "update-context section degraded after a committed update"
            );
            warnings.push(payment_types::UpdateContextWarning {
                section,
                code: code.to_string(),
                wallet: None,
                message: format!("{error:?}"),
            });
            None
        }
    }
}

/// A core response is only usable when it is a plain JSON body; anything else (a redirect, a
/// file) means the section cannot contribute to the composite.
fn json_body<T>(response: ApplicationResponse<T>) -> Result<T, errors::ApiErrorResponse> {
    match response {
        ApplicationResponse::Json(payload) | ApplicationResponse::JsonWithHeaders((payload, _)) => {
            Ok(payload)
        }
        _ => Err(errors::ApiErrorResponse::InternalServerError),
    }
}

#[instrument(skip_all, fields(payment_id))]
#[allow(clippy::too_many_arguments)]
pub async fn payments_update_context(
    state: SessionState,
    req_state: ReqState,
    platform: domain::Platform,
    profile_id: Option<id_type::ProfileId>,
    payment_id: id_type::PaymentId,
    req: payment_types::PaymentsUpdateContextRequest,
    header_payload: hyperswitch_domain_models::payments::HeaderPayload,
) -> RouterResponse<payment_types::PaymentsUpdateContextResponse> {
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    validate_request(&req)?;

    // ── 1 · Update — hard gate ───────────────────────────────────────────────
    // Same core and operation the standalone update route uses. A failure here fails the whole
    // request: nothing downstream would be meaningful, and no state has changed yet.
    let update_response = Box::pin(payments::payments_core::<
        api_types::UpdatePostConfirm,
        payment_types::PaymentsResponse,
        _,
        _,
        _,
        payments::PaymentData<api_types::UpdatePostConfirm>,
    >(
        state.clone(),
        req_state.clone(),
        platform.clone(),
        profile_id.clone(),
        payments::PaymentUpdate,
        to_payments_request(&req, &payment_id),
        AuthFlow::Merchant,
        payments::CallConnectorAction::Trigger,
        None,
        None,
        header_payload.clone(),
        None,
    ))
    .await?;

    let payment = json_body(update_response).map_err(|error| report!(error))?;

    // ── 2 · Fan-out — concurrent, each section degradable ─────────────────────
    // Both read the intent and so must follow the committed update, but neither reads the
    // other's output.
    let wants_sessions = !req.wallets.is_empty();
    let wants_payment_methods = req.include_payment_methods;

    let session_future = async {
        if wants_sessions {
            Some(
                Box::pin(payments::payments_core::<
                    api_types::Session,
                    payment_types::PaymentsSessionResponse,
                    _,
                    _,
                    _,
                    payments::PaymentData<api_types::Session>,
                >(
                    state.clone(),
                    req_state.clone(),
                    platform.clone(),
                    profile_id.clone(),
                    payments::PaymentSession,
                    payment_types::PaymentsSessionRequest {
                        payment_id: payment_id.clone(),
                        client_secret: None,
                        wallets: req.wallets.clone(),
                        merchant_connector_details: None,
                    },
                    AuthFlow::Merchant,
                    payments::CallConnectorAction::Trigger,
                    None,
                    None,
                    header_payload.clone(),
                    None,
                ))
                .await,
            )
        } else {
            None
        }
    };

    let payment_methods_future = async {
        if wants_payment_methods {
            Some(
                Box::pin(pm_client::list_payment_methods_client(
                    state.clone(),
                    platform.clone(),
                    payment_id.clone(),
                    // API-key authenticated; there is no client secret to validate.
                    None,
                ))
                .await,
            )
        } else {
            None
        }
    };

    let (session_result, payment_methods_result) =
        futures::future::join(session_future, payment_methods_future).await;

    // ── 3 · Assemble — degraded sections become null plus a warning ───────────
    let mut warnings = Vec::new();

    let session_tokens = session_result
        .and_then(|result| {
            degrade_on_error(
                result,
                payment_types::UpdateContextSection::SessionTokens,
                SESSION_TOKENS_FAILED,
                &mut warnings,
            )
        })
        .and_then(|response| {
            degrade_on_error(
                json_body(response),
                payment_types::UpdateContextSection::SessionTokens,
                SESSION_TOKENS_FAILED,
                &mut warnings,
            )
        })
        .map(|session| session.session_token);

    let payment_methods = payment_methods_result
        .and_then(|result| {
            degrade_on_error(
                result,
                payment_types::UpdateContextSection::PaymentMethods,
                PAYMENT_METHODS_FAILED,
                &mut warnings,
            )
        })
        .and_then(|response| {
            degrade_on_error(
                json_body(response),
                payment_types::UpdateContextSection::PaymentMethods,
                PAYMENT_METHODS_FAILED,
                &mut warnings,
            )
        });

    // Carried through from the update rather than re-minted, so it cannot disagree with what the
    // standalone update returns.
    let sdk_authorization = payment.sdk_authorization.clone();

    Ok(ApplicationResponse::Json(
        payment_types::PaymentsUpdateContextResponse {
            payment: Box::new(payment),
            session_tokens,
            payment_methods,
            sdk_authorization,
            // Reserved: the external-vault session is out of scope for this version.
            vault_sdk_authorization: None,
            warnings,
        },
    ))
}
