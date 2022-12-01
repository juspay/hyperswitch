use core::time::Duration;

use error_stack::{IntoReport, ResultExt};

use crate::{
    configs::settings::{Locker, Proxy},
    core::errors::{self, CustomResult},
};

const HTTP_PROXY: &str = "ROUTER_HTTP_PROXY";
const HTTPS_PROXY: &str = "ROUTER_HTTPS_PROXY";

enum ProxyType {
    Http,
    Https,
}

impl ProxyType {
    fn get_proxy_url(&self, proxy: &Proxy) -> Option<String> {
        use std::env::var;

        match self {
            ProxyType::Http => var(HTTP_PROXY)
                .or_else(|_| proxy.http_url.clone().ok_or(()))
                .ok(),
            ProxyType::Https => var(HTTPS_PROXY)
                .or_else(|_| proxy.https_url.clone().ok_or(()))
                .ok(),
        }
    }
}

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
pub(super) fn create_client(
    proxy: &Proxy,
    should_bypass_proxy: bool,
    request_time_out: u64,
) -> CustomResult<reqwest::Client, errors::ApiClientError> {
    let mut client_builder = reqwest::Client::builder();

    if !should_bypass_proxy {
        if let Some(url) = ProxyType::Http.get_proxy_url(proxy) {
            client_builder = client_builder.proxy(
                reqwest::Proxy::http(url)
                    .into_report()
                    .change_context(errors::ApiClientError::InvalidProxyConfiguration)
                    .attach_printable_lazy(|| "HTTP proxy configuration error")?,
            );
        }
        if let Some(url) = ProxyType::Https.get_proxy_url(proxy) {
            client_builder = client_builder.proxy(
                reqwest::Proxy::https(url)
                    .into_report()
                    .change_context(errors::ApiClientError::InvalidProxyConfiguration)
                    .attach_printable_lazy(|| "HTTPS proxy configuration error")?,
            );
        }
    }

    let duration = Duration::from_secs(request_time_out);
    client_builder = client_builder.timeout(duration);

    client_builder
        .build()
        .into_report()
        .change_context(errors::ApiClientError::ClientConstructionFailed)
        .attach_printable_lazy(|| "Error with client library")
}

// TODO: Move to env variable
pub(super) fn proxy_bypass_urls(locker: &Locker) -> Vec<String> {
    let locker_host = locker.host.to_owned();
    let basilisk_host = locker.basilisk_host.to_owned();
    vec![
        format!("{locker_host}/card/addCard"),
        format!("{locker_host}/card/getCard"),
        format!("{locker_host}/card/deleteCard"),
        format!("{basilisk_host}/tokenize"),
        format!("{basilisk_host}/tokenize/get"),
        format!("{basilisk_host}/tokenize/delete"),
        format!("{basilisk_host}/tokenize/delete/token"),
    ]
}
