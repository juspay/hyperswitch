//! Input validation for merchant-controlled URLs routed through the configured egress proxy.

use std::{net::IpAddr, str::FromStr};

use error_stack::{Report, ResultExt};
use url::{Host, Url};

use crate::{
    errors::{CustomResult, ValidationError},
    fp_utils::when,
};

/// An absolute HTTP or HTTPS URL, or an empty webhook destination.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct SafeOutboundUrl(Option<Url>);

impl SafeOutboundUrl {
    /// Validate an already parsed URL.
    pub fn from_url(url: Url) -> CustomResult<Self, ValidationError> {
        when(!matches!(url.scheme(), "http" | "https"), || {
            Err(Report::new(ValidationError::InvalidValue {
                message: "URL must use HTTP or HTTPS".to_string(),
            }))
        })?;
        let address = match url.host() {
            Some(Host::Ipv4(address)) => Some(IpAddr::V4(address)),
            Some(Host::Ipv6(address)) => Some(IpAddr::V6(address)),
            Some(Host::Domain(_)) => None,
            None => None,
        };
        when(
            address.is_some_and(|address| !Self::is_public_ip(address)),
            || {
                Err(Report::new(ValidationError::InvalidValue {
                    message: "URL IP address must be publicly routable".to_string(),
                }))
            },
        )?;
        Ok(Self(Some(url)))
    }

    fn is_public_ip(address: IpAddr) -> bool {
        // Judge IPv4-mapped IPv6 as IPv4, including private and metadata addresses.
        let address = address.to_canonical();
        ip_network::IpNetwork::from(address).is_global()
    }

    /// Whether this value clears a webhook destination.
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    /// Reject a destination that matches the configured egress proxy bypass hosts.
    pub fn validate_proxy_bypass_hosts(
        &self,
        bypass_proxy_hosts: Option<&str>,
    ) -> CustomResult<(), ValidationError> {
        let host = self.0.as_ref().and_then(Url::host);
        let is_blocked = bypass_proxy_hosts
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.trim_matches('.').is_empty())
            .any(|entry| match host.as_ref() {
                Some(Host::Domain(domain)) => {
                    let host = domain.trim_end_matches('.').to_ascii_lowercase();
                    let entry = entry.trim_matches('.').to_ascii_lowercase();
                    entry == "*"
                        || host == entry
                        || host
                            .strip_suffix(&entry)
                            .is_some_and(|prefix| prefix.ends_with('.'))
                }
                Some(Host::Ipv4(address)) => {
                    let address = IpAddr::V4(*address);
                    entry.parse::<IpAddr>().is_ok_and(|ip| ip == address)
                        || ip_network::IpNetwork::from_str_truncate(entry)
                            .is_ok_and(|network| network.contains(address))
                }
                Some(Host::Ipv6(address)) => {
                    let address = IpAddr::V6(*address);
                    entry.parse::<IpAddr>().is_ok_and(|ip| ip == address)
                        || ip_network::IpNetwork::from_str_truncate(entry)
                            .is_ok_and(|network| network.contains(address))
                }
                None => false,
            });
        when(is_blocked, || {
            Err(Report::new(ValidationError::InvalidValue {
                message: "URL matches a configured proxy bypass host".to_string(),
            }))
        })
    }

    /// Get the normalized URL string, or an empty string for a cleared webhook destination.
    pub fn get_string_repr(&self) -> &str {
        self.0.as_ref().map_or("", Url::as_str)
    }
}

impl FromStr for SafeOutboundUrl {
    type Err = Report<ValidationError>;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            Ok(Self(None))
        } else {
            Url::parse(value)
                .change_context(ValidationError::InvalidValue {
                    message: "URL could not be parsed".to_string(),
                })
                .and_then(Self::from_url)
        }
    }
}

impl TryFrom<String> for SafeOutboundUrl {
    type Error = Report<ValidationError>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl From<SafeOutboundUrl> for String {
    fn from(value: SafeOutboundUrl) -> Self {
        value.0.map(Self::from).unwrap_or_default()
    }
}

impl hyperswitch_masking::SerializableSecret for SafeOutboundUrl {}

#[cfg(test)]
mod tests {
    use super::SafeOutboundUrl;

    #[test]
    fn rejects_non_global_ip_literals() {
        for value in [
            "https://169.254.169.254/latest/meta-data/",
            "https://2852039166/latest/meta-data/",
            "https://0xa9fea9fe/latest/meta-data/",
            "https://[::ffff:169.254.169.254]/latest/meta-data/",
            "https://127.1/",
            "https://0177.0.0.1/",
            "https://2130706433/",
            "https://10.0.4.17/",
            "https://[::1]/",
            "https://[fc00::1]/",
            "https://[fe80::1]/",
        ] {
            assert!(
                value.parse::<SafeOutboundUrl>().is_err(),
                "accepted {value}"
            );
        }
    }

    #[test]
    fn accepts_public_ip_literals() {
        for value in [
            "http://8.8.8.8/",
            "https://8.8.8.8/",
            "https://[::ffff:8.8.8.8]/",
            "https://[2606:4700:4700::1111]/",
        ] {
            assert!(value.parse::<SafeOutboundUrl>().is_ok(), "rejected {value}");
        }
    }

    #[test]
    fn cleared_webhook_url_round_trips_without_becoming_a_request_destination() {
        let url: SafeOutboundUrl = serde_json::from_str("\"\"").expect("cleared webhook");
        assert!(url.is_empty());
        assert_eq!(url.get_string_repr(), "");
        assert_eq!(serde_json::to_string(&url).expect("cleared URL"), "\"\"");
    }

    #[test]
    fn typed_url_validates_syntax_and_preserves_the_json_string() {
        let url: SafeOutboundUrl =
            serde_json::from_str("\"https://merchant.example.com/hook\"").expect("valid typed URL");
        assert_eq!(url.get_string_repr(), "https://merchant.example.com/hook");
        assert_eq!(
            serde_json::to_string(&url).expect("serialized typed URL"),
            "\"https://merchant.example.com/hook\""
        );
        assert!(
            serde_json::from_str::<SafeOutboundUrl>("\"http://merchant.example.com/hook\"").is_ok()
        );
        for value in [
            "/hook",
            "file:///etc/passwd",
            "ftp://merchant.example.com/hook",
        ] {
            assert!(serde_json::from_value::<SafeOutboundUrl>(serde_json::json!(value)).is_err());
        }
    }

    #[test]
    fn leaves_non_bypassed_destinations_to_the_proxy() {
        for value in [
            "https://merchant.example.com/hook",
            "https://user:password@merchant.example.com/hook",
            "http://10.0.4.17.nip.io/hook",
            "https://cluster.local.example.com/hook",
            "https://notcluster.local/hook",
            "https://localhost.example.com/hook",
        ] {
            assert!(value.parse::<SafeOutboundUrl>().is_ok());
        }
    }
}
