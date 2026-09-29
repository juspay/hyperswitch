//! Destination checks for requests whose target was chosen by a merchant.
//!
//! Two layers, because an IP literal never reaches a resolver: [`check_destination`] judges the
//! url before the request is built, and [`PublicOnlyResolver`] judges the addresses a name
//! resolves to, so hyper only ever dials addresses that passed.

use std::net::{IpAddr, SocketAddr};

use common_utils::outbound_url::is_globally_routable;
use hyper::client::connect::dns::Name;
use reqwest::dns::{Addrs, Resolve, Resolving};
use router_env::logger;
use url::{Host, Url};

use super::metrics;

/// The destination is not publicly routable.
#[derive(Debug, thiserror::Error)]
#[error("outbound destination `{host}` is not allowed")]
pub struct DestinationBlocked {
    host: String,
}

/// A resolver that drops every non-public address before hyper sees the list.
#[derive(Clone, Copy, Debug, Default)]
pub struct PublicOnlyResolver;

impl Resolve for PublicOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        Box::pin(async move {
            let addresses = tokio::net::lookup_host((name.as_str(), 0)).await?.collect();
            let allowed = filter_addresses(name.as_str(), addresses)?;
            let addresses: Addrs = Box::new(allowed.into_iter());
            Ok::<Addrs, Box<dyn std::error::Error + Send + Sync>>(addresses)
        })
    }
}

/// The check that runs before a request is built. IP literals are judged here because they never
/// reach a resolver. Names are left to [`PublicOnlyResolver`], except when `resolve_here` is set:
/// behind an egress proxy the proxy resolves the name, so the lookup happens here instead.
pub async fn check_destination(url: &Url, resolve_here: bool) -> Result<(), DestinationBlocked> {
    let port = url.port_or_known_default().unwrap_or_default();
    match url.host() {
        None => Ok(()),
        Some(Host::Ipv4(address)) => check_literal(IpAddr::V4(address), port),
        Some(Host::Ipv6(address)) => check_literal(IpAddr::V6(address), port),
        Some(Host::Domain(name)) if resolve_here => {
            match tokio::net::lookup_host((name, port)).await {
                Ok(addresses) => filter_addresses(name, addresses.collect()).map(|_| ()),
                Err(error) => {
                    logger::warn!(
                        ?error,
                        host = name,
                        "pre-flight lookup of outbound destination failed"
                    );
                    Err(DestinationBlocked {
                        host: name.to_string(),
                    })
                }
            }
        }
        Some(Host::Domain(_)) => Ok(()),
    }
}

fn check_literal(address: IpAddr, port: u16) -> Result<(), DestinationBlocked> {
    filter_addresses(&address.to_string(), vec![SocketAddr::new(address, port)]).map(|_| ())
}

/// Keeps the publicly routable addresses and refuses the name only if none remain.
pub fn filter_addresses(
    host: &str,
    addresses: Vec<SocketAddr>,
) -> Result<Vec<SocketAddr>, DestinationBlocked> {
    let (allowed, blocked): (Vec<SocketAddr>, Vec<SocketAddr>) = addresses
        .into_iter()
        .partition(|address| is_globally_routable(address.ip()));

    match (blocked.is_empty(), allowed.is_empty()) {
        (true, _) => Ok(allowed),
        (false, allowed_is_empty) => {
            let blocked: Vec<IpAddr> = blocked.iter().map(SocketAddr::ip).collect();
            logger::warn!(
                host,
                ?blocked,
                "outbound destination resolved to a non-public address"
            );
            metrics::OUTBOUND_DESTINATION_BLOCKED
                .add(1, router_env::metric_attributes!(("host", host.to_owned())));
            match allowed_is_empty {
                true => Err(DestinationBlocked {
                    host: host.to_string(),
                }),
                false => Ok(allowed),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addresses(values: &[&str]) -> Vec<SocketAddr> {
        values
            .iter()
            .map(|value| SocketAddr::new(value.parse().expect("test address must parse"), 443))
            .collect()
    }

    #[test]
    fn all_public_addresses_pass_unchanged() {
        let input = addresses(&["8.8.8.8", "2001:4860:4860::8888"]);
        let output = filter_addresses("x.example", input.clone());
        assert_eq!(output.expect("must pass"), input);
    }

    #[test]
    fn private_addresses_are_dropped_and_public_ones_kept() {
        let input = addresses(&["8.8.8.8", "10.0.4.17", "169.254.169.254"]);
        let output = filter_addresses("x.example", input);
        assert_eq!(output.expect("must pass"), addresses(&["8.8.8.8"]));
    }

    #[test]
    fn a_name_with_only_private_addresses_is_refused() {
        let input = addresses(&["10.0.4.17", "127.0.0.1"]);
        assert!(filter_addresses("x.example", input).is_err());
    }

    #[test]
    fn ipv4_mapped_form_is_unwrapped() {
        let input = addresses(&["::ffff:10.0.4.17", "::ffff:127.0.0.1"]);
        assert!(filter_addresses("x.example", input).is_err());
    }

    #[tokio::test]
    async fn literal_hosts_are_judged_without_a_lookup() {
        let private = Url::parse("http://169.254.169.254/latest/meta-data/").expect("url");
        let public = Url::parse("https://8.8.8.8/hook").expect("url");
        let mapped = Url::parse("http://[::ffff:127.0.0.1]/").expect("url");

        assert!(check_destination(&private, false).await.is_err());
        assert!(check_destination(&public, false).await.is_ok());
        assert!(check_destination(&mapped, false).await.is_err());
    }

    #[tokio::test]
    async fn names_are_left_to_the_resolver_unless_asked() {
        let url = Url::parse("http://hooks.example/").expect("url");
        assert!(check_destination(&url, false).await.is_ok());
    }
}
