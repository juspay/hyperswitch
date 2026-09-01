//! Authentication.
//!
//! Modelled on the router's `AuthenticateAndFetch`: a route names the authentication it wants, as
//! a **required argument** to [`crate::services::server_wrap`]. That is the load-bearing property
//! — a route added later cannot silently skip authentication, because omitting the argument is a
//! compile error. [`NoAuth`] exists so that "this route is deliberately open" is a decision
//! someone wrote down and a reviewer can grep for.
//!
//! Unlike the router's trait, ours yields `()`. The router's returns a merchant context the
//! handler goes on to use — which is why it cannot be middleware. Ours has nothing to hand back,
//! and gains a type parameter the day it does.

use actix_web::http::header::HeaderMap;
use error_stack::report;
use hyperswitch_masking::PeekInterface;

use crate::{errors::AlertsError, state::AppState};

/// The header carrying the internal API key.
///
/// Spelled to match the rest of the repo (`router/src/lib.rs`, `subscriptions/src/helpers.rs`),
/// which each declare their own copy. It says "service-to-service" where the router's `api-key`
/// says "merchant credential", and this is the former.
pub const X_INTERNAL_API_KEY: &str = "X-Internal-Api-Key";

/// A way of authenticating a request.
///
/// `Sync` because implementations are passed as `&dyn Authenticate` into an async wrapper.
pub trait Authenticate: Sync {
    /// Authenticate a request, or reject it.
    fn authenticate(
        &self,
        request_headers: &HeaderMap,
        state: &AppState,
    ) -> error_stack::Result<(), AlertsError>;
}

/// Requires a valid internal API key in the [`X_INTERNAL_API_KEY`] header.
#[derive(Debug, Default)]
pub struct InternalApiKeyAuth;

impl Authenticate for InternalApiKeyAuth {
    fn authenticate(
        &self,
        request_headers: &HeaderMap,
        state: &AppState,
    ) -> error_stack::Result<(), AlertsError> {
        let supplied_key = request_headers
            .get(X_INTERNAL_API_KEY)
            .ok_or_else(|| {
                report!(AlertsError::Unauthorized)
                    .attach_printable("Internal API key header not present")
            })?
            .to_str()
            .map_err(|_| {
                report!(AlertsError::Unauthorized)
                    .attach_printable("Internal API key header is not valid UTF-8")
            })?;

        let configured_key = state.conf.auth.get_inner().internal_api_key.peek();

        // A plain comparison, matching the router's own `AdminApiAuth`. Introducing the
        // workspace's first constant-time comparison dependency here would be inconsistent
        // without being meaningfully safer for an internal endpoint.
        if supplied_key != configured_key {
            Err(report!(AlertsError::Unauthorized)
                .attach_printable("Internal API key authentication failure"))?;
        }

        Ok(())
    }
}

/// Performs no authentication.
///
/// Only for routes that are deliberately public. Prefer keeping such routes in their own
/// unguarded scope — see [`crate::health_check`] — so that "unauthenticated" is visible in the
/// route tree rather than only at the call site.
#[derive(Debug, Default)]
pub struct NoAuth;

impl Authenticate for NoAuth {
    fn authenticate(
        &self,
        _request_headers: &HeaderMap,
        _state: &AppState,
    ) -> error_stack::Result<(), AlertsError> {
        Ok(())
    }
}
