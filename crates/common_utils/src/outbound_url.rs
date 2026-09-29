//! Validation for outbound URLs whose destination is influenced by external input.

use std::{
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use error_stack::{report, Report, ResultExt};
use url::{Host, Url};

use crate::errors::{ParsingError, ValidationError};

/// A URL that is safe to use as the destination of a server side request: absolute, `http` or
/// `https`, free of credentials, and addressed by a public domain name or a globally routable IP
/// literal.
///
/// The only ways to build one are [`FromStr`], [`TryFrom<String>`] and [`SafeOutboundUrl::from_url`],
/// so a value of this type has always been validated.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct SafeOutboundUrl(Url);

impl SafeOutboundUrl {
    /// Validate an already parsed [`Url`].
    pub fn from_url(url: Url) -> Result<Self, Report<ValidationError>> {
        validate_scheme(&url)
            .and_then(|()| validate_credentials(&url))
            .and_then(|()| validate_host(&url))
            .map(|()| Self(url))
    }

    /// Get the string representation of the url.
    pub fn get_string_repr(&self) -> &str {
        self.0.as_str()
    }

    /// Get the host of the url: a public domain name or a globally routable IP literal.
    pub fn host_str(&self) -> Option<&str> {
        self.0.host_str()
    }

    /// Get the scheme of the url.
    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    /// Get the port of the url, falling back to the default port for its scheme.
    pub fn port_or_known_default(&self) -> Option<u16> {
        self.0.port_or_known_default()
    }

    /// Get the inner url.
    pub fn into_inner(self) -> Url {
        self.0
    }
}

impl FromStr for SafeOutboundUrl {
    type Err = Report<ValidationError>;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Url::parse(value)
            .change_context(ValidationError::InvalidValue {
                message: "url could not be parsed".to_string(),
            })
            .and_then(Self::from_url)
    }
}

impl TryFrom<String> for SafeOutboundUrl {
    type Error = Report<ParsingError>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value).change_context(ParsingError::UrlParsingError)
    }
}

impl fmt::Display for SafeOutboundUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.as_str())
    }
}

impl hyperswitch_masking::SerializableSecret for SafeOutboundUrl {}

impl serde::Serialize for SafeOutboundUrl {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

fn invalid(message: &str) -> ValidationError {
    ValidationError::InvalidValue {
        message: message.to_string(),
    }
}

fn validate_scheme(url: &Url) -> Result<(), Report<ValidationError>> {
    match matches!(url.scheme(), "http" | "https") && !url.cannot_be_a_base() {
        true => Ok(()),
        false => Err(report!(invalid("url scheme must be http or https"))),
    }
}

fn validate_credentials(url: &Url) -> Result<(), Report<ValidationError>> {
    match url.username().is_empty() && url.password().is_none() {
        true => Ok(()),
        false => Err(report!(invalid("url must not contain credentials"))),
    }
}

fn validate_host(url: &Url) -> Result<(), Report<ValidationError>> {
    match url.host() {
        Some(Host::Domain(domain)) => validate_domain(domain),
        Some(Host::Ipv4(address)) => validate_ip_literal(IpAddr::V4(address)),
        Some(Host::Ipv6(address)) => validate_ip_literal(IpAddr::V6(address)),
        None => Err(report!(invalid("url must have a host"))),
    }
}

fn validate_ip_literal(address: IpAddr) -> Result<(), Report<ValidationError>> {
    match is_globally_routable(address) {
        true => Ok(()),
        false => Err(report!(invalid(
            "url host must be a publicly routable address"
        ))),
    }
}

fn validate_domain(domain: &str) -> Result<(), Report<ValidationError>> {
    let normalised = domain.trim_end_matches('.').to_ascii_lowercase();
    let is_loopback_name = normalised == "localhost" || normalised.ends_with(".localhost");
    let is_multi_label = normalised.contains('.');

    match !normalised.is_empty() && is_multi_label && !is_loopback_name {
        true => Ok(()),
        false => Err(report!(invalid("url host must be a public domain name"))),
    }
}

/// Whether an address is reachable on the public internet, as opposed to loopback, private,
/// link-local, multicast or documentation space, as classified by the standard library.
///
/// The rule the url type applies to IP literals and the resolver applies to resolved names.
/// `to_canonical` first, so an IPv4-mapped address (`::ffff:10.0.4.17`) is judged as IPv4.
pub fn is_globally_routable(address: IpAddr) -> bool {
    match address.to_canonical() {
        IpAddr::V4(address) => is_globally_routable_v4(address),
        IpAddr::V6(address) => is_globally_routable_v6(address),
    }
}

fn is_globally_routable_v4(address: Ipv4Addr) -> bool {
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_multicast()
        || address.is_broadcast()
        || address.is_documentation())
}

fn is_globally_routable_v6(address: Ipv6Addr) -> bool {
    !(address.is_unspecified()
        || address.is_loopback()
        || address.is_multicast()
        || address.is_unique_local()
        || address.is_unicast_link_local())
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::*;

    #[test_case("https://merchant.example.com/hooks")]
    #[test_case("https://api.example.com:8443/x?a=b#c")]
    #[test_case("http://plain.example.com/")]
    #[test_case("https://xn--fsq.com/")]
    #[test_case("https://8.8.8.8/")]
    #[test_case("http://[2001:4860:4860::8888]/")]
    fn accepts_public_urls(value: &str) {
        assert!(SafeOutboundUrl::from_str(value).is_ok());
    }

    #[test_case("http://127.0.0.1")]
    #[test_case("http://0177.0.0.1")]
    #[test_case("http://2130706433")]
    #[test_case("http://127.1")]
    #[test_case("http://10.0.4.17:8080/admin")]
    #[test_case("http://169.254.169.254/latest/meta-data/")]
    #[test_case("http://[::1]/")]
    #[test_case("http://[::ffff:127.0.0.1]/")]
    #[test_case("http://[fc00::1]/")]
    #[test_case("http://[fe80::1]/")]
    #[test_case("http://[fe80::1%25eth0]/")]
    #[test_case("http://localhost/")]
    #[test_case("http://foo.localhost/")]
    #[test_case("http://singlelabel/")]
    #[test_case("http://user@10.0.0.1/")]
    #[test_case("https://good.example.com@169.254.169.254/")]
    #[test_case("file:///etc/passwd")]
    #[test_case("gopher://example.com/")]
    #[test_case("ftp://example.com/")]
    #[test_case("data:text/html,x")]
    #[test_case("//no-scheme/")]
    #[test_case("https://")]
    fn rejects_unsafe_urls(value: &str) {
        assert!(SafeOutboundUrl::from_str(value).is_err());
    }

    #[test_case("8.8.8.8", true)]
    #[test_case("1.0.0.1", true)]
    #[test_case("0.0.0.0", false)]
    #[test_case("10.255.255.255", false)]
    #[test_case("127.0.0.1", false)]
    #[test_case("169.254.0.1", false)]
    #[test_case("172.15.255.255", true)]
    #[test_case("172.16.0.0", false)]
    #[test_case("172.31.255.255", false)]
    #[test_case("172.32.0.0", true)]
    #[test_case("192.0.2.1", false)]
    #[test_case("192.168.1.1", false)]
    #[test_case("198.51.100.1", false)]
    #[test_case("203.0.113.1", false)]
    #[test_case("223.255.255.255", true)]
    #[test_case("224.0.0.1", false)]
    #[test_case("255.255.255.255", false)]
    #[test_case("2001:4860:4860::8888", true)]
    #[test_case("::", false)]
    #[test_case("::1", false)]
    #[test_case("::ffff:127.0.0.1", false)]
    #[test_case("::ffff:10.0.4.17", false)]
    #[test_case("::ffff:8.8.8.8", true)]
    #[test_case("fc00::1", false)]
    #[test_case("fdff::1", false)]
    #[test_case("fe80::1", false)]
    #[test_case("ff02::1", false)]
    fn classifies_addresses(value: &str, expected: bool) {
        let address = value.parse::<IpAddr>().expect("test address must parse");
        assert_eq!(is_globally_routable(address), expected);
    }
}
