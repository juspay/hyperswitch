//! Incoming webhook source IP validation
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
};

use actix_web::http::header::HeaderMap;
use api_models::enums::Connector;
use common_utils::errors::ErrorSwitch;
use ip_network::IpNetwork;
use router_env::logger;

use super::source_verification::NetworkVerification;
use crate::{core::errors, headers};

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub(super) enum VerificationFailure {
    InvalidConfig,
    InvalidConnectorName,
    MissingHeader,
    InvalidHeader,
    IpNotAllowed,
}

impl ErrorSwitch<errors::ApiErrorResponse> for VerificationFailure {
    fn switch(&self) -> errors::ApiErrorResponse {
        match self {
            Self::InvalidConfig => errors::ApiErrorResponse::InternalServerError,
            Self::InvalidConnectorName => errors::ApiErrorResponse::IncorrectConnectorNameGiven,
            Self::MissingHeader | Self::InvalidHeader | Self::IpNotAllowed => {
                errors::ApiErrorResponse::WebhookAuthenticationFailed
            }
        }
    }
}

/// Example Superposition config for `webhooks.incoming_webhooks_allowed_ip_ranges`:
/// ```json
/// { "envoy": ["198.51.100.10", "195.35.90.0/25", "2001:db8::/64"] }
/// ```
/// Each connector maps to a nonempty list of individual IPs or CIDR ranges.
/// An empty object disables IP checks for all connectors.
fn parse_allowed_ip_ranges(
    value: serde_json::Value,
) -> Result<HashMap<Connector, Vec<IpNetwork>>, VerificationFailure> {
    let entries: HashMap<Connector, Vec<String>> =
        serde_json::from_value(value).map_err(|error| {
            logger::error!(
                ?error,
                "Failed to deserialize incoming webhook allowed IP ranges"
            );
            VerificationFailure::InvalidConfig
        })?;
    entries
        .into_iter()
        .map(|(connector, entries)| {
            if entries.is_empty() {
                Err(VerificationFailure::InvalidConfig)
            } else {
                let allowed_ip_ranges = entries
                    .into_iter()
                    .map(|entry| {
                        let entry = entry.trim();
                        IpNetwork::from_str_truncate(entry)
                            .or_else(|_| entry.parse::<IpAddr>().map(IpNetwork::from))
                            .map_err(|_| VerificationFailure::InvalidConfig)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((connector, allowed_ip_ranges))
            }
        })
        .collect()
}

pub(super) fn validate_source_ip_against_allowed_ip_ranges(
    policy: serde_json::Value,
    connector_name: &str,
    request_headers: &HeaderMap,
) -> Result<NetworkVerification, VerificationFailure> {
    let policy = parse_allowed_ip_ranges(policy)?;
    let connector = connector_name
        .parse::<Connector>()
        .map_err(|_| VerificationFailure::InvalidConnectorName)?;
    let connector_allowed_ip_ranges = policy.get(&connector).map(Vec::as_slice);
    validate_source_ip(connector_allowed_ip_ranges, request_headers)
}

fn validate_source_ip(
    allowed_ip_ranges: Option<&[IpNetwork]>,
    request_headers: &HeaderMap,
) -> Result<NetworkVerification, VerificationFailure> {
    match allowed_ip_ranges {
        None => Ok(NetworkVerification::NotRequired),
        Some(allowed_ip_ranges) => {
            let mut values = request_headers.get_all(headers::TRUE_CLIENT_IP);
            let value = values.next().ok_or(VerificationFailure::MissingHeader)?;
            // Only a single header should be present for client IP
            if values.next().is_some() {
                Err(VerificationFailure::InvalidHeader)
            } else {
                let source = value
                    .to_str()
                    .ok()
                    .and_then(|value| {
                        value
                            .parse::<IpAddr>()
                            .or_else(|_| value.parse::<SocketAddr>().map(|address| address.ip()))
                            .ok()
                    })
                    .ok_or(VerificationFailure::InvalidHeader)?;

                if allowed_ip_ranges
                    .iter()
                    .any(|ip_range| ip_range.contains(source))
                {
                    Ok(NetworkVerification::Verified)
                } else {
                    Err(VerificationFailure::IpNotAllowed)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
    use api_models::enums::Connector;
    use common_utils::errors::ErrorSwitch;
    use external_services::superposition::Config;
    use ip_network::IpNetwork;
    use serde_json::json;

    use super::{
        parse_allowed_ip_ranges, validate_source_ip, validate_source_ip_against_allowed_ip_ranges,
        NetworkVerification, VerificationFailure,
    };
    use crate::{
        core::{configs::dimension_config::IncomingWebhookAllowedIpRanges, errors},
        headers,
    };

    fn headers_with(values: &[&str]) -> HeaderMap {
        let mut result = HeaderMap::new();
        for value in values {
            result.append(
                HeaderName::from_static(headers::TRUE_CLIENT_IP),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        result
    }

    #[test]
    fn accepts_ipv4_with_or_without_a_port() {
        let allowed_ip_ranges = ["198.51.100.10/32".parse().unwrap()];
        for value in [
            "198.51.100.10",
            "198.51.100.10:443",
            "198.51.100.10:46532",
            "198.51.100.10:65535",
        ] {
            assert_eq!(
                validate_source_ip(Some(&allowed_ip_ranges), &headers_with(&[value])),
                Ok(NetworkVerification::Verified)
            );
        }
        for value in ["198.51.100.11", "198.51.100.11:46532"] {
            assert_eq!(
                validate_source_ip(Some(&allowed_ip_ranges), &headers_with(&[value])),
                Err(VerificationFailure::IpNotAllowed)
            );
        }
    }

    #[test]
    fn checks_cidr_boundaries_and_multiple_ip_ranges() {
        let allowed_ip_ranges = [
            "195.35.90.0/25".parse().unwrap(),
            "195.35.91.0/25".parse().unwrap(),
        ];
        for address in [
            "195.35.90.0",
            "195.35.90.127",
            "195.35.91.1",
            "195.35.91.126",
        ] {
            assert_eq!(
                validate_source_ip(
                    Some(&allowed_ip_ranges),
                    &headers_with(&[&format!("{address}:46532")])
                ),
                Ok(NetworkVerification::Verified)
            );
        }
        for address in ["195.35.89.255", "195.35.90.128", "195.35.91.128"] {
            assert_eq!(
                validate_source_ip(
                    Some(&allowed_ip_ranges),
                    &headers_with(&[&format!("{address}:46532")])
                ),
                Err(VerificationFailure::IpNotAllowed)
            );
        }
    }

    #[test]
    fn accepts_ipv6_with_or_without_a_port() {
        let allowed_ip_ranges = ["2001:db8::/64".parse().unwrap()];
        for value in ["2001:db8::10", "[2001:db8::10]:46532", "2001:db8::10:443"] {
            assert_eq!(
                validate_source_ip(Some(&allowed_ip_ranges), &headers_with(&[value])),
                Ok(NetworkVerification::Verified)
            );
        }
        for value in ["2001:db8:1::10", "[2001:db8:1::10]:46532"] {
            assert_eq!(
                validate_source_ip(Some(&allowed_ip_ranges), &headers_with(&[value])),
                Err(VerificationFailure::IpNotAllowed)
            );
        }
    }

    #[test]
    fn rejects_missing_malformed_and_repeated_headers() {
        let allowed_ip_ranges = ["198.51.100.10/32".parse().unwrap()];
        assert_eq!(
            validate_source_ip(Some(&allowed_ip_ranges), &HeaderMap::new()),
            Err(VerificationFailure::MissingHeader)
        );
        for value in [
            "",
            "198.51.100.10:65536",
            "198.51.100.10:abc",
            "host.example:46532",
            "host.example",
            "198.51.100.10,198.51.100.11",
            "198.51.100.10:46532,198.51.100.11:46532",
            " 198.51.100.10:46532",
            "198.51.100.10:46532 ",
            "2001:db8::10:46532",
            "[2001:db8::10]:65536",
            "[2001:db8::10]:abc",
        ] {
            assert_eq!(
                validate_source_ip(Some(&allowed_ip_ranges), &headers_with(&[value])),
                Err(VerificationFailure::InvalidHeader),
                "accepted {value:?}"
            );
        }
        assert_eq!(
            validate_source_ip(
                Some(&allowed_ip_ranges),
                &headers_with(&["198.51.100.10:46532", "198.51.100.10:46532"])
            ),
            Err(VerificationFailure::InvalidHeader)
        );
    }

    #[test]
    fn ignores_other_client_ip_headers() {
        let allowed_ip_ranges = ["198.51.100.10/32".parse().unwrap()];
        let mut request_headers = HeaderMap::new();
        request_headers.insert(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("198.51.100.10"),
        );
        assert_eq!(
            validate_source_ip(Some(&allowed_ip_ranges), &request_headers),
            Err(VerificationFailure::MissingHeader)
        );
    }

    #[test]
    fn handles_header_names_case_insensitively_and_rejects_non_utf8_values() {
        let allowed_ip_ranges = ["198.51.100.10/32".parse().unwrap()];
        let mut request_headers = HeaderMap::new();
        request_headers.insert(
            HeaderName::from_bytes(b"True-Client-IP").unwrap(),
            HeaderValue::from_static("198.51.100.10:46532"),
        );
        assert_eq!(
            validate_source_ip(Some(&allowed_ip_ranges), &request_headers),
            Ok(NetworkVerification::Verified)
        );
        request_headers.insert(
            HeaderName::from_static(headers::TRUE_CLIENT_IP),
            HeaderValue::from_bytes(&[0xff]).unwrap(),
        );
        assert_eq!(
            validate_source_ip(Some(&allowed_ip_ranges), &request_headers),
            Err(VerificationFailure::InvalidHeader)
        );
    }

    #[test]
    fn leaves_unconfigured_connectors_unchanged_and_empty_lists_deny_all() {
        assert_eq!(
            validate_source_ip(None, &HeaderMap::new()),
            Ok(NetworkVerification::NotRequired)
        );
        assert_eq!(
            validate_source_ip(None, &headers_with(&["invalid"])),
            Ok(NetworkVerification::NotRequired)
        );
        assert_eq!(
            validate_source_ip(Some(&[]), &headers_with(&["198.51.100.10:46532"])),
            Err(VerificationFailure::IpNotAllowed)
        );
    }

    #[test]
    fn reads_superposition_ip_range_arrays() {
        let policy = parse_allowed_ip_ranges(json!({
            "envoy": ["198.51.100.10", "195.35.90.0/25", "2001:db8::/64"]
        }))
        .unwrap();
        assert_eq!(policy[&Connector::Envoy].len(), 3);
        assert_eq!(
            policy[&Connector::Envoy][0],
            "198.51.100.10/32".parse::<IpNetwork>().unwrap()
        );
        assert!(parse_allowed_ip_ranges(json!({})).unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_superposition_policy() {
        for value in [
            json!(null),
            json!([]),
            json!({"envoy": []}),
            json!({"envoy": "198.51.100.10"}),
            json!({"envoy": [""]}),
            json!({"envoy": ["198.51.100.10", "invalid"]}),
            json!({"envoy": ["198.51.100.10/33"]}),
            json!({"envoy": ["198.51.100.10:46532"]}),
            json!({"unknown_connector": ["198.51.100.10"]}),
        ] {
            assert_eq!(
                parse_allowed_ip_ranges(value.clone()),
                Err(VerificationFailure::InvalidConfig),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn absent_config_skips_verification_but_invalid_policy_is_rejected() {
        let headers = headers_with(&["198.51.100.10:46532"]);
        assert_eq!(
            validate_source_ip_against_allowed_ip_ranges(
                IncomingWebhookAllowedIpRanges::default_value(),
                "envoy",
                &HeaderMap::new()
            ),
            Ok(NetworkVerification::NotRequired)
        );
        assert_eq!(
            validate_source_ip_against_allowed_ip_ranges(json!({"envoy": []}), "envoy", &headers),
            Err(VerificationFailure::InvalidConfig)
        );
        assert!(matches!(
            VerificationFailure::InvalidConfig.switch(),
            errors::ApiErrorResponse::InternalServerError
        ));
        for failure in [
            VerificationFailure::MissingHeader,
            VerificationFailure::InvalidHeader,
            VerificationFailure::IpNotAllowed,
        ] {
            assert!(matches!(
                failure.switch(),
                errors::ApiErrorResponse::WebhookAuthenticationFailed
            ));
        }
    }

    #[test]
    fn rejects_invalid_connector_names_even_with_an_empty_policy() {
        for policy in [json!({}), json!({"envoy": ["198.51.100.10"]})] {
            assert_eq!(
                validate_source_ip_against_allowed_ip_ranges(
                    policy,
                    "unknown_connector",
                    &HeaderMap::new()
                ),
                Err(VerificationFailure::InvalidConnectorName)
            );
        }
        assert!(matches!(
            VerificationFailure::InvalidConnectorName.switch(),
            errors::ApiErrorResponse::IncorrectConnectorNameGiven
        ));
    }

    #[test]
    fn each_request_uses_the_current_superposition_policy() {
        let headers = headers_with(&["198.51.100.10:46532"]);
        assert_eq!(
            validate_source_ip_against_allowed_ip_ranges(
                json!({"envoy": ["198.51.100.10"]}),
                "envoy",
                &headers
            ),
            Ok(NetworkVerification::Verified)
        );
        assert_eq!(
            validate_source_ip_against_allowed_ip_ranges(
                json!({"envoy": ["198.51.100.11"]}),
                "envoy",
                &headers
            ),
            Err(VerificationFailure::IpNotAllowed)
        );
        assert_eq!(
            validate_source_ip_against_allowed_ip_ranges(
                json!({"envoy": ["198.51.100.11"]}),
                "stripe",
                &HeaderMap::new()
            ),
            Ok(NetworkVerification::NotRequired)
        );
        assert_eq!(
            validate_source_ip_against_allowed_ip_ranges(json!({}), "envoy", &HeaderMap::new()),
            Ok(NetworkVerification::NotRequired)
        );
    }
}
