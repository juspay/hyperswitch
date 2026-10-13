//! Source verification for incoming webhooks that the router performs itself, independent of
//! the connector and of the execution path.
//!
//! A connector usually verifies the source of a webhook from its payload (signature, shared
//! secret or a verification call). Connectors that rely on mTLS cannot do that: the client
//! certificate is validated by the infrastructure layer, which marks such requests with a
//! header. That transport-level verification is handled here.

use actix_web::http::header::HeaderMap;
use error_stack::report;

use crate::{
    core::errors::{self, RouterResult},
    headers,
    routes::SessionState,
    services::logger,
};

/// Transport-level source verification for connectors that require mTLS.
///
/// Fails with a webhook authentication error when the connector requires mTLS and the request
/// does not carry the mTLS verification header. The header is set by the infrastructure layer
/// only for requests received on the mTLS endpoints; it is stripped from requests arriving on
/// any other endpoint.
///
/// This needs nothing but the connector name and the request headers, so it is meant to run as
/// soon as the connector is resolved, to reject the webhook before any further processing.
pub(super) fn verify_mtls(
    state: &SessionState,
    connector_name: &str,
    request_headers: &HeaderMap,
) -> RouterResult<()> {
    if is_mtls_required(state, connector_name)
        && !is_mtls_verification_header_present(request_headers)
    {
        logger::warn!(
            connector = %connector_name,
            "Source Verification Failed: connector requires mTLS but the webhook was not received over a verified mTLS endpoint"
        );

        return Err(
            report!(errors::ApiErrorResponse::WebhookAuthenticationFailed).attach_printable(
                "Connector requires mTLS but the webhook was not received over a verified mTLS endpoint",
            ),
        );
    }

    Ok(())
}

/// Whether incoming webhooks of the connector must be received over a verified mTLS endpoint,
/// as configured in `webhooks.mtls_required_connectors`.
fn is_mtls_required(state: &SessionState, connector_name: &str) -> bool {
    connector_name
        .parse::<api_models::enums::Connector>()
        .is_ok_and(|connector| {
            state
                .conf
                .webhooks
                .mtls_required_connectors
                .contains(&connector)
        })
}

/// Checks for exactly one mTLS verification header with the value `true`; anything else counts
/// as absent.
fn is_mtls_verification_header_present(request_headers: &HeaderMap) -> bool {
    let mut header_values = request_headers.get_all(headers::X_CLIENT_MTLS_VERIFIED);
    matches!(
        (header_values.next(), header_values.next()),
        (Some(value), None) if value.as_bytes() == b"true"
    )
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};

    use super::is_mtls_verification_header_present;
    use crate::headers;

    fn headers_with(values: &[&'static str]) -> HeaderMap {
        let mut request_headers = HeaderMap::new();
        for value in values {
            request_headers.append(
                HeaderName::from_static(headers::X_CLIENT_MTLS_VERIFIED),
                HeaderValue::from_static(value),
            );
        }
        request_headers
    }

    #[test]
    fn accepts_single_true_value() {
        assert!(is_mtls_verification_header_present(&headers_with(&[
            "true"
        ])));
    }

    #[test]
    fn rejects_missing_header() {
        assert!(!is_mtls_verification_header_present(&headers_with(&[])));
    }

    #[test]
    fn rejects_other_values() {
        for value in ["false", "TRUE", "1", "true,true", " true", ""] {
            assert!(!is_mtls_verification_header_present(&headers_with(&[
                value
            ])));
        }
    }

    #[test]
    fn rejects_repeated_header() {
        assert!(!is_mtls_verification_header_present(&headers_with(&[
            "true", "true"
        ])));
    }
}
