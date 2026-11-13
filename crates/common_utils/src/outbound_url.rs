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

#[derive(Clone, Debug)]
struct HostEntry {
    domain: String,
    address: Option<IpAddr>,
    network: Option<ip_network::IpNetwork>,
}

impl HostEntry {
    fn parse(entry: &str) -> Self {
        Self {
            domain: entry.trim_matches('.').to_ascii_lowercase(),
            address: entry.parse::<IpAddr>().ok(),
            network: ip_network::IpNetwork::from_str_truncate(entry).ok(),
        }
    }

    fn matches_domain(&self, domain: &str) -> bool {
        self.domain == "*"
            || domain == self.domain
            || domain
                .strip_suffix(&self.domain)
                .is_some_and(|prefix| prefix.ends_with('.'))
    }

    fn matches_address(&self, address: IpAddr) -> bool {
        self.address == Some(address)
            || self
                .network
                .is_some_and(|network| network.contains(address))
    }
}

/// A comma-separated list of domains, IP addresses or CIDR ranges, parsed once. A domain entry
/// matches the domain itself and all of its subdomains.
#[derive(Clone, Debug, Default)]
pub struct HostMatcher {
    entries: Vec<HostEntry>,
}

impl HostMatcher {
    /// Parse the list, ignoring empty entries.
    pub fn parse(list: Option<&str>) -> Self {
        let entries = list
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.trim_matches('.').is_empty())
            .map(HostEntry::parse)
            .collect();
        Self { entries }
    }

    /// Whether the host matches any entry of the list.
    pub fn matches(&self, host: Option<&Host<&str>>) -> bool {
        match host {
            Some(Host::Domain(domain)) => {
                let domain = domain.trim_end_matches('.').to_ascii_lowercase();
                self.entries
                    .iter()
                    .any(|entry| entry.matches_domain(&domain))
            }
            Some(Host::Ipv4(address)) => self
                .entries
                .iter()
                .any(|entry| entry.matches_address(IpAddr::V4(*address))),
            Some(Host::Ipv6(address)) => self
                .entries
                .iter()
                .any(|entry| entry.matches_address(IpAddr::V6(*address))),
            None => false,
        }
    }
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
        when(
            HostMatcher::parse(bypass_proxy_hosts).matches(self.0.host().as_ref()),
            || {
                Err(Report::new(ValidationError::InvalidValue {
                    message: "URL matches a configured proxy bypass host".to_string(),
                }))
            },
        )
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
