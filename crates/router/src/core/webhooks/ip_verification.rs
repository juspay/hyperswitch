//! Incoming webhook source IP validation
use std::net::{IpAddr, SocketAddr};

use actix_web::http::header::HeaderMap;
use common_utils::errors::ErrorSwitch;
use ip_network::IpNetwork;
use router_env::logger;

use super::source_verification::VerificationStatus;
use crate::core::{configs::dimension_config::IncomingWebhookIpRangesConfig, errors};

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(super) enum VerificationFailure {
    InvalidConfig,
    MissingHeaderConfig,
    MissingHeader,
    InvalidHeader,
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
fn parse_allowed_ip_ranges(entries: Vec<String>) -> Result<Vec<IpNetwork>, VerificationFailure> {
    if entries.is_empty() {
        logger::error!("Incoming webhook allowed IP ranges must not be empty");
        Err(VerificationFailure::InvalidConfig)
    } else {
        entries
            .into_iter()
            .map(|entry| {
                let entry = entry.trim();
                entry
                    .parse::<IpNetwork>()
                    .or_else(|_| entry.parse::<IpAddr>().map(IpNetwork::from))
                    .map_err(|error| {
                        logger::error!(
                            ?error,
                            ip_range = %entry,
                            "Invalid incoming webhook allowed IP range"
                        );
                        VerificationFailure::InvalidConfig
                    })
            })
            .collect()
    }
}

pub(super) fn validate_source_ip_against_allowed_ip_ranges(
    mut policy: IncomingWebhookIpRangesConfig,
    connector_name: &str,
    request_headers: &HeaderMap,
    header_name: Option<&str>,
) -> Result<VerificationStatus, VerificationFailure> {
    let entry = policy.remove(connector_name).ok_or_else(|| {
        logger::error!(
            connector = %connector_name,
            "Incoming webhook allowed IP ranges are not configured for the connector"
        );
        VerificationFailure::InvalidConfig
    })?;
    let entries = serde_json::from_value(entry).map_err(|error| {
        logger::error!(
            connector = %connector_name,
            ?error,
            "Failed to parse incoming webhook IP range list"
        );
        VerificationFailure::InvalidConfig
    })?;
    let connector_allowed_ip_ranges = parse_allowed_ip_ranges(entries)?;
    validate_source_ip(&connector_allowed_ip_ranges, request_headers, header_name).inspect_err(
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
    allowed_ip_ranges: &[IpNetwork],
    request_headers: &HeaderMap,
    header_name: Option<&str>,
) -> Result<VerificationStatus, VerificationFailure> {
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
        Err(VerificationFailure::InvalidHeader)
    } else {
        let value = value.to_str().map_err(|error| {
            logger::warn!(
                ?error,
                header = header_name,
                "Incoming webhook source IP header is not valid UTF-8"
            );
            VerificationFailure::InvalidHeader
        })?;
        let source = value
            .parse::<IpAddr>()
            .or_else(|_| value.parse::<SocketAddr>().map(|address| address.ip()))
            .map_err(|error| {
                logger::warn!(
                    ?error,
                    header = header_name,
                    "Incoming webhook source IP header is not an IP or socket address"
                );
                VerificationFailure::InvalidHeader
            })?;

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
            Err(VerificationFailure::IpNotAllowed)
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
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
    ) -> Result<VerificationStatus, VerificationFailure> {
        validate_source_ip_against_allowed_ip_ranges(
            serde_json::from_value(policy).map_err(|_| VerificationFailure::InvalidConfig)?,
            "envoy",
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
            "195.35.90.0",
            "195.35.90.127:46532",
            "2001:db8::10",
            "[2001:db8::10]:46532",
        ] {
            assert_eq!(
                verify(policy.clone(), &headers_with(&[value])),
                Ok(VerificationStatus::Verified),
                "rejected {value}"
            );
        }
        for value in ["198.51.100.11", "195.35.90.128:46532", "2001:db8:1::10"] {
            assert_eq!(
                verify(policy.clone(), &headers_with(&[value])),
                Err(VerificationFailure::IpNotAllowed),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn rejects_missing_malformed_and_repeated_headers() {
        let policy = json!({"envoy": ["198.51.100.10"]});
        assert_eq!(
            verify(policy.clone(), &HeaderMap::new()),
            Err(VerificationFailure::MissingHeader)
        );
        for values in [
            vec!["not-an-ip"],
            vec!["198.51.100.10:65536"],
            vec!["198.51.100.10,198.51.100.11"],
            vec!["198.51.100.10", "198.51.100.10"],
        ] {
            assert_eq!(
                verify(policy.clone(), &headers_with(&values)),
                Err(VerificationFailure::InvalidHeader)
            );
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static(TEST_SOURCE_IP_HEADER),
            HeaderValue::from_bytes(&[0xff]).unwrap(),
        );
        assert_eq!(
            verify(policy, &headers),
            Err(VerificationFailure::InvalidHeader)
        );
    }

    #[test]
    fn rejects_missing_or_invalid_allowed_ranges() {
        for policy in [
            json!(null),
            json!([]),
            json!({}),
            json!({"other": ["198.51.100.10"]}),
            json!({"envoy": null}),
            json!({"envoy": []}),
            json!({"envoy": "198.51.100.10"}),
            json!({"envoy": [123]}),
            json!({"envoy": ["198.51.100.10", "invalid"]}),
            json!({"envoy": ["198.51.100.10/33"]}),
            json!({"envoy": ["198.51.100.10/24"]}),
            json!({"envoy": ["2001:db8::1/64"]}),
        ] {
            assert_eq!(
                verify(policy.clone(), &headers_with(&["198.51.100.10"])),
                Err(VerificationFailure::InvalidConfig),
                "accepted {policy}"
            );
        }
    }

    #[test]
    fn invalid_entries_do_not_affect_other_connectors() {
        for invalid_ranges in [json!(null), json!([123]), json!(["invalid"])] {
            assert_eq!(
                verify(
                    json!({"envoy": ["198.51.100.10"], "other": invalid_ranges}),
                    &headers_with(&["198.51.100.10"])
                ),
                Ok(VerificationStatus::Verified)
            );
        }
    }

    #[test]
    fn requires_and_uses_only_the_configured_source_ip_header() {
        let ranges = ["198.51.100.10/32".parse().unwrap()];
        let mut headers = headers_with(&["198.51.100.10"]);
        for name in [None, Some("")] {
            assert_eq!(
                validate_source_ip(&ranges, &headers, name),
                Err(VerificationFailure::MissingHeaderConfig)
            );
        }
        let custom_header = "x-custom-source-ip";
        assert_eq!(
            validate_source_ip(&ranges, &headers, Some(custom_header)),
            Err(VerificationFailure::MissingHeader)
        );
        headers.insert(
            HeaderName::from_static(custom_header),
            HeaderValue::from_static("198.51.100.10"),
        );
        assert_eq!(
            validate_source_ip(&ranges, &headers, Some(custom_header)),
            Ok(VerificationStatus::Verified)
        );
    }
}
