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
    core::response::ApplicationResponse,
    errors::{types::ApiErrorResponse, ObservabilityError},
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
    Fut: Future<Output = error_stack::Result<Q, ObservabilityError>>,
    Q: IntoApplicationResponse,
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
        Ok(response) => response.into_http_response(),
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

/// Existing JSON core handlers retain their return types; explicit response variants opt into
/// status/header control without a route-local renderer.
pub trait IntoApplicationResponse {
    fn into_http_response(self) -> HttpResponse;
}

impl<T: Serialize> IntoApplicationResponse for T {
    fn into_http_response(self) -> HttpResponse {
        HttpResponse::Ok().json(self)
    }
}

impl<T: Serialize> IntoApplicationResponse for ApplicationResponse<T> {
    fn into_http_response(self) -> HttpResponse {
        match self {
            Self::JsonWithHeaders { body, headers } => {
                let mut response = HttpResponse::Ok();
                for (name, value) in headers {
                    response.append_header((name, value));
                }
                response.json(body)
            }
            Self::NoContentWithHeaders { headers } => {
                let mut response = HttpResponse::NoContent();
                for (name, value) in headers {
                    response.append_header((name, value));
                }
                response.finish()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        body::to_bytes,
        http::{
            header::{HeaderMap, HeaderValue, SET_COOKIE},
            StatusCode,
        },
    };
    use serde_json::json;

    #[actix_web::test]
    async fn json_with_headers_preserves_body_and_repeated_headers() {
        let mut headers = HeaderMap::new();
        headers.append(SET_COOKIE, HeaderValue::from_static("first=one; HttpOnly"));
        headers.append(SET_COOKIE, HeaderValue::from_static("second=two; HttpOnly"));
        let response = ApplicationResponse::JsonWithHeaders {
            body: json!({"ok": true}),
            headers,
        }
        .into_http_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get_all(SET_COOKIE).count(), 2);
        let body = to_bytes(response.into_body()).await;
        assert!(matches!(body, Ok(ref bytes) if bytes.as_ref() == br#"{"ok":true}"#));
    }

    #[actix_web::test]
    async fn no_content_with_headers_has_no_json_body() {
        let response = ApplicationResponse::<()>::NoContentWithHeaders {
            headers: HeaderMap::new(),
        }
        .into_http_response();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let body = to_bytes(response.into_body()).await;
        assert!(matches!(body, Ok(ref bytes) if bytes.is_empty()));
    }
}
