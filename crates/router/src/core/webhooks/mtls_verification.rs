//! mTLS verification for incoming webhooks for connectors that use mTLS for source verification
//!
//! Validates the X_CLIENT_MTLS_VERIFIED header set by infra layer

use actix_web::http::header::HeaderMap;
use error_stack::report;

use crate::{
    core::{
        errors::{self, RouterResult},
        metrics,
    },
    headers,
    routes::SessionState,
    services::logger,
};

/// Transport-level source verification for connectors that require mTLS.
///
/// Fails with a webhook authentication error when the connector requires mTLS and the request
/// does not carry the mTLS verification header. The header is set by the infras layer
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
        metrics::WEBHOOK_MTLS_VERIFICATION_FAILURE_COUNT.add(
            1,
            router_env::metric_attributes!(("connector", connector_name.to_string())),
        );
        logger::warn!(
            connector = %connector_name,
            "Source Verification Failed: connector requires mTLS but the webhook was not received over a verified mTLS endpoint"
        );

        Err(
            report!(errors::ApiErrorResponse::WebhookAuthenticationFailed).attach_printable(
                "Connector requires mTLS but the webhook was not received over a verified mTLS endpoint",
            ),
        )
    } else {
        Ok(())
    }
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
    header_values
        .next()
        .is_some_and(|value| value.as_bytes() == b"true")
        && header_values.next().is_none()
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
