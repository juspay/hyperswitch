//! Incoming webhook source IP validation
use std::net::IpAddr;

use actix_web::http::header::HeaderMap;
use common_enums::connector_enums::Connector;
use common_utils::errors::{CustomResult, ErrorSwitch};
use error_stack::{report, ResultExt};
use hyperswitch_domain_models::webhooks::IncomingWebhookIpRange;
use router_env::logger;

use super::source_verification::VerificationStatus;
use crate::{
    core::{configs::dimension_config::IncomingWebhookIpRangesConfig, errors},
    types::transformers::ForeignTryFrom,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr, thiserror::Error)]
#[strum(serialize_all = "snake_case")]
pub(super) enum VerificationFailure {
    #[error("Invalid incoming webhook allowed IP ranges config")]
    InvalidConfig,
    #[error("Incoming webhook source IP header is not configured")]
    MissingHeaderConfig,
    #[error("Incoming webhook source IP header is missing")]
    MissingHeader,
    #[error("Incoming webhook source IP header is invalid")]
    InvalidHeader,
    #[error("Incoming webhook source IP is outside the allowed ranges")]
    IpNotAllowed,
}

impl ErrorSwitch<errors::ApiErrorResponse> for VerificationFailure {
    fn switch(&self) -> errors::ApiErrorResponse {
        match self {
            Self::InvalidConfig => errors::ApiErrorResponse::InternalServerError,
            Self::MissingHeaderConfig
            | Self::MissingHeader
            | Self::InvalidHeader
            | Self::IpNotAllowed => errors::ApiErrorResponse::WebhookAuthenticationFailed,
        }
    }
}

/// Example Superposition config for `webhooks.incoming_webhooks_allowed_ip_ranges`:
/// ```json
/// { "envoy": ["198.51.100.10", "195.35.90.0/25", "2001:db8::/64"] }
/// ```
/// Each connector maps to a nonempty list of individual IPs or CIDR ranges.
/// CIDRs must use the range's base address, with no host bits set.
/// Connectors requiring IP verification must have an entry when verification is enabled.
pub(super) fn validate_source_ip_against_allowed_ip_ranges(
    policy: IncomingWebhookIpRangesConfig,
    connector_name: Connector,
    request_headers: &HeaderMap,
    header_name: Option<&str>,
) -> CustomResult<VerificationStatus, VerificationFailure> {
    let connector_allowed_ip_ranges = policy
        .get(&connector_name)
        .filter(|ranges| !ranges.is_empty())
        .ok_or_else(|| {
            logger::error!(
                connector = %connector_name,
                "Incoming webhook allowed IP ranges are missing or empty for the connector"
            );
            VerificationFailure::InvalidConfig
        })?;
    validate_source_ip(connector_allowed_ip_ranges, request_headers, header_name).inspect_err(
        |reason| {
            logger::warn!(
                connector = %connector_name,
                ?reason,
                header = ?header_name,
                header_config = "incoming_webhooks.source_ip_header",
                "Incoming webhook source IP verification rejected the request"
            );
        },
    )
}

fn validate_source_ip(
    allowed_ip_ranges: &[IncomingWebhookIpRange],
    request_headers: &HeaderMap,
    header_name: Option<&str>,
) -> CustomResult<VerificationStatus, VerificationFailure> {
    let header_name = header_name
        .filter(|name| !name.is_empty())
        .ok_or(VerificationFailure::MissingHeaderConfig)?;
    let mut values = request_headers.get_all(header_name);
    let value = values.next().ok_or(VerificationFailure::MissingHeader)?;
    // Only a single header should be present for client IP
    if values.next().is_some() {
        logger::warn!(
            header = header_name,
            "Incoming webhook has multiple source IP header values"
        );
        Err(report!(VerificationFailure::InvalidHeader))
    } else {
        let source =
            IpAddr::foreign_try_from(value).change_context(VerificationFailure::InvalidHeader)?;

        if allowed_ip_ranges
            .iter()
            .any(|ip_range| ip_range.contains(source))
        {
            Ok(VerificationStatus::Verified)
        } else {
            logger::warn!(
                source_ip = %source,
                allowed_range_count = allowed_ip_ranges.len(),
                "Incoming webhook source IP is outside the allowed ranges"
            );
            Err(report!(VerificationFailure::IpNotAllowed))
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
    use common_enums::connector_enums::Connector;
    use common_utils::errors::CustomResult;
    use error_stack::ResultExt;
    use serde_json::json;

    use super::{
        validate_source_ip, validate_source_ip_against_allowed_ip_ranges, VerificationFailure,
        VerificationStatus,
    };

    const TEST_SOURCE_IP_HEADER: &str = "x-test-source-ip";

    fn headers_with(values: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(
                HeaderName::from_static(TEST_SOURCE_IP_HEADER),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        headers
    }

    fn verify(
        policy: serde_json::Value,
        headers: &HeaderMap,
    ) -> CustomResult<VerificationStatus, VerificationFailure> {
        validate_source_ip_against_allowed_ip_ranges(
            serde_json::from_value(policy).change_context(VerificationFailure::InvalidConfig)?,
            Connector::Envoy,
            headers,
            Some(TEST_SOURCE_IP_HEADER),
        )
    }

    #[test]
    fn validates_ips_with_or_without_ports_against_allowed_ranges() {
        let policy = json!({"envoy": ["198.51.100.10", "195.35.90.0/25", "2001:db8::/64"]});
        for value in [
            "198.51.100.10",
            "198.51.100.10:46532",
            "195.35.90.127:46532",
            "2001:db8::10",
            "[2001:db8::10]:46532",
        ] {
            assert_eq!(
                verify(policy.clone(), &headers_with(&[value])).unwrap(),
                VerificationStatus::Verified,
                "rejected {value}"
            );
        }
        for value in ["198.51.100.11", "195.35.90.128:46532", "2001:db8:1::10"] {
            assert_eq!(
                *verify(policy.clone(), &headers_with(&[value]))
                    .unwrap_err()
                    .current_context(),
                VerificationFailure::IpNotAllowed,
                "accepted {value}"
            );
        }
    }

    #[test]
    fn rejects_missing_malformed_and_repeated_headers() {
        let policy = json!({"envoy": ["198.51.100.10"]});
        assert_eq!(
            *verify(policy.clone(), &HeaderMap::new())
                .unwrap_err()
                .current_context(),
            VerificationFailure::MissingHeader
        );
        for values in [vec!["not-an-ip"], vec!["198.51.100.10", "198.51.100.10"]] {
            let error = verify(policy.clone(), &headers_with(&values)).unwrap_err();
            assert_eq!(*error.current_context(), VerificationFailure::InvalidHeader);
        }
    }

    #[test]
    fn rejects_missing_or_empty_connector_ranges() {
        for policy in [
            json!({}),
            json!({"stripe": ["198.51.100.10"]}),
            json!({"envoy": []}),
        ] {
            assert_eq!(
                *verify(policy.clone(), &headers_with(&["198.51.100.10"]))
                    .unwrap_err()
                    .current_context(),
                VerificationFailure::InvalidConfig,
                "accepted {policy}"
            );
        }
    }

    #[test]
    fn requires_and_uses_only_the_configured_source_ip_header() {
        let ranges = ["198.51.100.10/32".parse().unwrap()];
        let mut headers = headers_with(&["198.51.100.10"]);
        for name in [None, Some("")] {
            assert_eq!(
                *validate_source_ip(&ranges, &headers, name)
                    .unwrap_err()
                    .current_context(),
                VerificationFailure::MissingHeaderConfig
            );
        }
        let custom_header = "x-custom-source-ip";
        assert_eq!(
            *validate_source_ip(&ranges, &headers, Some(custom_header))
                .unwrap_err()
                .current_context(),
            VerificationFailure::MissingHeader
        );
        headers.insert(
            HeaderName::from_static(custom_header),
            HeaderValue::from_static("198.51.100.10"),
        );
        assert_eq!(
            validate_source_ip(&ranges, &headers, Some(custom_header)).unwrap(),
            VerificationStatus::Verified
        );
    }
}
