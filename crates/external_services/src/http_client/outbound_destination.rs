//! DNS-resolution guard for merchant-controlled outbound destinations (webhook URLs, proxy
//! destinations), preventing SSRF via domain names that resolve to non-public addresses.

use std::time::Duration;

use common_utils::{
    errors::{CustomResult, ValidationError},
    fp_utils::when,
    outbound_url::{is_global_ip, SafeOutboundUrl},
};
use error_stack::{Report, ResultExt};
use router_env::logger;
use url::Host;

const DNS_LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);

fn invalid_dns_destination() -> ValidationError {
    ValidationError::InvalidValue {
        message: "URL hostname must resolve to publicly routable IP addresses".to_string(),
    }
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
                let addresses =
                    tokio::time::timeout(DNS_LOOKUP_TIMEOUT, tokio::net::lookup_host((host, 0)))
                        .await
                        .change_context(invalid_dns_destination())?
                        .change_context(invalid_dns_destination())?;

                let mut addresses = addresses.peekable();
                let is_blocked = addresses.peek().is_none()
                    || addresses.any(|address| !is_global_ip(address.ip()));

                when(is_blocked, || Err(Report::new(invalid_dns_destination())))
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
