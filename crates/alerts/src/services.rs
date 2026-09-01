//! The request wrapper every route goes through.
//!
//! Scaled-down `server_wrap`, keeping the property that matters — authentication is a **required
//! argument**, so it cannot be forgotten — and dropping what the router carries for reasons that
//! do not apply here: tenancy, `api_locking`, flow metrics, and API-event billing.
//!
//! What it does carry is the request id and structured logging, so that every log line emitted
//! while handling a request can be correlated. For a service whose job is talking to flaky third
//! parties, that is the difference between "an alert did not arrive" and knowing why.

use std::{fmt::Debug, future::Future};

use actix_web::{FromRequest, HttpRequest, HttpResponse, ResponseError};
use common_utils::errors::ErrorSwitch;
use router_env::{instrument, tracing, RequestId};
use serde::Serialize;

use crate::{
    auth::Authenticate,
    errors::{types::ApiErrorResponse, AlertsError},
    logger,
    state::AppState,
};

/// Authenticate, run the handler, and render whatever comes back.
///
/// `auth` is positional and required. Adding a route means naming its authentication; there is no
/// default and no way to leave it out.
#[instrument(skip_all)]
pub async fn server_wrap<T, Q, F, Fut>(
    state: AppState,
    request: &HttpRequest,
    payload: T,
    handler: F,
    auth: &dyn Authenticate,
) -> HttpResponse
where
    F: FnOnce(AppState, T) -> Fut,
    Fut: Future<Output = error_stack::Result<Q, AlertsError>>,
    Q: Serialize + Debug,
    T: Debug,
{
    let request_id = RequestId::extract(request)
        .await
        .map(|id| id.as_str().to_owned())
        .unwrap_or_default();
    let path = request.path().to_owned();

    if let Err(error) = auth.authenticate(request.headers(), &state) {
        // The rejection is logged with enough to find the caller and nothing that would help an
        // attacker: never the supplied key, and never its length.
        logger::warn!(
            request_id = %request_id,
            path = %path,
            peer_address = ?request.peer_addr(),
            error = ?error,
            "Request rejected: authentication failed"
        );
        return ErrorSwitch::<ApiErrorResponse>::switch(error.current_context()).error_response();
    }

    match handler(state, payload).await {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(error) => {
            // The full report — every `attach_printable` on the way up — goes to the log. The
            // client gets only what `ErrorSwitch` produces.
            logger::error!(
                request_id = %request_id,
                path = %path,
                error = ?error,
                "Request failed"
            );
            ErrorSwitch::<ApiErrorResponse>::switch(error.current_context()).error_response()
        }
    }
}
