//! Input validation for merchant-controlled URLs routed through the configured egress proxy.

use std::net::IpAddr;

use error_stack::Report;
use url::{Host, Url};

use crate::{
    errors::{CustomResult, ValidationError},
    fp_utils::when,
};

/// Whether the given IP address is publicly routable (not private, loopback, link-local, or a
/// cloud metadata address).
pub fn is_global_ip(address: IpAddr) -> bool {
    // Judge IPv4-mapped IPv6 as IPv4, including private and metadata addresses.
    let address = address.to_canonical();
    ip_network::IpNetwork::from(address).is_global()
}

/// An absolute HTTP or HTTPS URL with a publicly routable IP literal, if supplied.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(into = "String")]
pub struct SafeOutboundUrl(Url);

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
            address.is_some_and(|address| !is_global_ip(address)),
            || {
                Err(Report::new(ValidationError::InvalidValue {
                    message: "URL IP address must be publicly routable".to_string(),
                }))
            },
        )?;
        Ok(Self(url))
    }

    /// The URL's host, for callers that need to branch on literal vs. domain-name destinations
    /// (e.g. to resolve a domain name's DNS records before using it as a destination).
    pub fn host(&self) -> Option<Host<&str>> {
        self.0.host()
    }

    /// Reject a destination that matches the configured egress proxy bypass hosts.
    pub fn validate_proxy_bypass_hosts(
        &self,
        bypass_proxy_hosts: Option<&str>,
    ) -> CustomResult<(), ValidationError> {
        let host = self.0.host();
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

    /// Get the normalized URL string.
    pub fn get_string_repr(&self) -> &str {
        self.0.as_str()
    }
}

impl<'de> serde::Deserialize<'de> for SafeOutboundUrl {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let url = <Url as serde::Deserialize<'de>>::deserialize(deserializer)?;
        Self::from_url(url).map_err(serde::de::Error::custom)
    }
}

impl From<SafeOutboundUrl> for String {
    fn from(value: SafeOutboundUrl) -> Self {
        value.0.into()
    }
}

impl hyperswitch_masking::SerializableSecret for SafeOutboundUrl {}

#[cfg(test)]
mod tests {
    use super::SafeOutboundUrl;

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
            "",
            "/hook",
            "file:///etc/passwd",
            "ftp://merchant.example.com/hook",
        ] {
            assert!(serde_json::from_value::<SafeOutboundUrl>(serde_json::json!(value)).is_err());
        }
    }
}
