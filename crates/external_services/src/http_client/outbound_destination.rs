//! DNS-resolution guard for outbound destinations (webhook URLs, proxy destinations and other
//! public-internet requests), preventing SSRF via domain names that resolve to non-public addresses.

use std::{
    io,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use common_utils::{
    errors::{CustomResult, ValidationError},
    fp_utils::when,
    outbound_url::{is_global_ip, SafeOutboundUrl},
};
use error_stack::{Report, ResultExt};
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use router_env::logger;
use url::Host;

use super::metrics;

const DNS_LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);

fn invalid_dns_destination() -> ValidationError {
    ValidationError::InvalidValue {
        message: "URL hostname must resolve to publicly routable IP addresses".to_string(),
    }
}

/// Raised by [`GlobalOnlyResolver`] when a hostname resolves to a non-public address.
#[derive(Debug, thiserror::Error)]
#[error("destination host {host} resolves to a non-public address")]
pub struct DestinationNotPublic {
    host: String,
}

/// DNS resolver that rejects hostnames resolving to non-public addresses at connection time, so
/// the validated address is the connected address.
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalOnlyResolver;

impl Resolve for GlobalOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addresses = lookup_addresses(&host).await?;
            ensure_public(&host, &addresses)?;
            let resolved: Addrs = Box::new(addresses.into_iter());
            Ok(resolved)
        })
    }
}

/// Whether any of the failure's causes is a [`DestinationNotPublic`].
pub fn is_destination_not_public(error: &(dyn std::error::Error + 'static)) -> bool {
    std::iter::successors(Some(error), |err| err.source())
        .any(|err| err.is::<DestinationNotPublic>())
}

/// Rejects a URL whose host is an IP literal that is not publicly routable. Connections to IP
/// literals never reach the DNS resolver, so they are checked here instead.
pub fn ensure_public_url_host(url: &url::Url) -> Result<(), DestinationNotPublic> {
    let literal = match url.host() {
        Some(Host::Ipv4(address)) => Some(IpAddr::V4(address)),
        Some(Host::Ipv6(address)) => Some(IpAddr::V6(address)),
        Some(Host::Domain(_)) | None => None,
    };
    literal.map_or(Ok(()), |address| {
        ensure_public(&address.to_string(), &[SocketAddr::new(address, 0)])
    })
}

fn ensure_public(host: &str, addresses: &[SocketAddr]) -> Result<(), DestinationNotPublic> {
    when(contains_non_global(addresses), || report_blocked(host))
}

fn report_blocked(host: &str) -> Result<(), DestinationNotPublic> {
    metrics::OUTBOUND_DESTINATION_BLOCKED
        .add(1, router_env::metric_attributes!(("host", host.to_owned())));
    logger::warn!(
        host,
        "outbound destination resolves to a non-public address"
    );
    Err(DestinationNotPublic {
        host: host.to_owned(),
    })
}

async fn lookup_addresses(host: &str) -> io::Result<Vec<SocketAddr>> {
    let resolved = tokio::time::timeout(DNS_LOOKUP_TIMEOUT, tokio::net::lookup_host((host, 0)))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "DNS lookup timed out"))??;
    let addresses: Vec<SocketAddr> = resolved.collect();
    when(addresses.is_empty(), || {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "DNS lookup returned no addresses",
        ))
    })?;
    Ok(addresses)
}

fn contains_non_global(addresses: &[SocketAddr]) -> bool {
    addresses.iter().any(|address| !is_global_ip(address.ip()))
}

/// Validate proxy bypass rules and resolve hostnames before using a merchant-supplied URL as an
/// outbound destination.
pub async fn validate_destination(
    url: &SafeOutboundUrl,
    bypass_proxy_hosts: Option<&str>,
) -> CustomResult<(), ValidationError> {
    let outcome: CustomResult<(), ValidationError> = async {
        url.validate_proxy_bypass_hosts(bypass_proxy_hosts)?;

        // IP literals were validated during parsing; only domain names need DNS resolution.
        match url.host() {
            Some(Host::Domain(host)) => {
                let addresses = lookup_addresses(host)
                    .await
                    .change_context(invalid_dns_destination())?;

                when(contains_non_global(&addresses), || {
                    Err(Report::new(invalid_dns_destination()))
                })
            }
            _ => Ok(()),
        }
    }
    .await;

    outcome.inspect_err(|error| {
        logger::warn!(
            destination = url.get_string_repr(),
            ?error,
            "rejected outbound destination"
        );
    })
}
