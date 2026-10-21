//! Attribute HTTP client failures to the configured proxy without treating connector
//! timeouts or ordinary upstream responses as proxy outages.

use std::{error::Error, time::Duration};

use common_utils::outbound_url::matches_proxy_bypass_hosts;
use hyperswitch_interfaces::types::Proxy;

// Bound DNS and all address attempts together. This runs only after a connection
// failure or timeout, never before a healthy connector request, and sends no HTTP request.
const PROXY_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

/// Typed evidence retained across HTTP client and connector error conversions.
#[derive(Debug)]
pub struct ConfirmedProxyFailure(pub(super) &'static str);

impl ConfirmedProxyFailure {
    /// Stable reason recorded by callers protecting shadow traffic.
    pub fn reason(&self) -> &'static str {
        self.0
    }
}

fn connection_proxy<'a>(error: &reqwest::Error, proxy: &'a Proxy) -> Option<&'a str> {
    let destination = error.url()?;
    if matches_proxy_bypass_hosts(destination, proxy.bypass_proxy_hosts.as_deref()) {
        return None;
    }
    match destination.scheme() {
        "http" => proxy.http_url.as_deref(),
        "https" => proxy.https_url.as_deref(),
        _ => None,
    }
}

async fn connection_failure_reason(proxy_url: &str) -> Option<&'static str> {
    let endpoint = url::Url::parse(proxy_url).ok()?;
    if !matches!(endpoint.scheme(), "http" | "https") {
        return None;
    }
    let host = match endpoint.host()? {
        url::Host::Domain(host) => host.to_owned(),
        url::Host::Ipv4(address) => address.to_string(),
        url::Host::Ipv6(address) => address.to_string(),
    };
    let port = endpoint.port_or_known_default()?;
    let check = async {
        let addresses = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|_| "proxy_dns_failure")?;
        let mut attempts = tokio::task::JoinSet::new();
        for address in addresses {
            attempts.spawn(tokio::net::TcpStream::connect(address));
        }
        if attempts.is_empty() {
            return Err("proxy_dns_failure");
        }
        // One healthy address is sufficient, including when another IPv4/IPv6
        // address cannot be reached. Do not mistake address ordering for an outage.
        while let Some(result) = attempts.join_next().await {
            if matches!(result, Ok(Ok(_))) {
                return Ok(());
            }
        }
        Err("proxy_connection_failure")
    };
    match tokio::time::timeout(PROXY_CHECK_TIMEOUT, check).await {
        Ok(Ok(())) => None,
        Ok(Err(reason)) => Some(reason),
        Err(_) => Some("proxy_connection_timeout"),
    }
}

/// Return a reason only for a confirmed proxy authentication or endpoint failure.
///
/// A timeout alone is ambiguous. Reqwest's overall deadline can also hide which
/// connection stage timed out, so check the actual proxy endpoint for both timeout
/// and connection errors. A reachable proxy returns `None`, including when an
/// upstream connector times out or its TLS handshake fails. Body errors are ignored.
pub(super) async fn failure_reason(error: &reqwest::Error, proxy: &Proxy) -> Option<&'static str> {
    if !error.is_connect() && !error.is_timeout() {
        return None;
    }
    let proxy_url = connection_proxy(error, proxy)?;
    // Reqwest 0.11 exposes CONNECT authentication failures as this exact source
    // error, rather than an HTTP response. A real CONNECT regression test covers it.
    if std::iter::successors(error.source(), |source| (*source).source())
        .any(|source| source.to_string() == "proxy authentication required")
    {
        return Some("proxy_authentication_required");
    }
    connection_failure_reason(proxy_url).await
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task::JoinHandle,
    };

    use super::*;

    // Production client construction uses a shared cache with a non-blocking write
    // lock. Serialize these two cold-cache integration tests, not the proxy mocks.
    static HTTP_CLIENT_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    async fn unused_endpoint() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let address = listener.local_addr().expect("local address");
        drop(listener);
        format!("http://{address}")
    }

    async fn serve_proxy(response: &'static [u8]) -> (String, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let endpoint = format!("http://{}", listener.local_addr().expect("local address"));
        let task = tokio::spawn(async move {
            loop {
                let (mut connection, _) = listener.accept().await.expect("accept");
                tokio::spawn(async move {
                    let mut buffer = [0; 4096];
                    if connection.read(&mut buffer).await.unwrap_or_default() != 0 {
                        if response.is_empty() {
                            // Keep the connection alive until the client's read timeout.
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        } else {
                            let _ = connection.write_all(response).await;
                        }
                    }
                });
            }
        });
        (endpoint, task)
    }

    fn client(endpoint: &str) -> reqwest::Client {
        reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(endpoint).expect("valid proxy"))
            .timeout(Duration::from_secs(1))
            .build()
            .expect("client")
    }

    fn proxy(endpoint: String) -> Proxy {
        Proxy {
            http_url: Some(endpoint.clone()),
            https_url: Some(endpoint),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn stopped_proxy_is_a_confirmed_connection_failure() {
        let endpoint = unused_endpoint().await;
        let error = client(&endpoint)
            .get("https://connector.example/")
            .send()
            .await
            .expect_err("proxy is stopped");
        assert_eq!(
            failure_reason(&error, &proxy(endpoint)).await,
            Some("proxy_connection_failure")
        );
    }

    #[tokio::test]
    async fn confirmed_proxy_evidence_survives_connector_error_conversion() {
        let _guard = HTTP_CLIENT_TEST_LOCK.lock().await;
        let endpoint = unused_endpoint().await;
        let request = common_utils::request::Request::new(
            common_utils::request::Method::Get,
            "https://connector.example/",
        );
        let error = super::super::send_request(&proxy(endpoint), request, Some(1))
            .await
            .expect_err("proxy is stopped")
            .change_context(
                hyperswitch_interfaces::errors::ConnectorError::ProcessingStepFailed(None),
            );
        assert_eq!(
            error
                .downcast_ref::<ConfirmedProxyFailure>()
                .map(ConfirmedProxyFailure::reason),
            Some("proxy_connection_failure")
        );
        assert!(error.downcast_ref::<reqwest::Error>().is_some());
    }

    #[tokio::test]
    async fn http_client_keeps_generic_connector_timeouts_without_proxy_evidence() {
        let _guard = HTTP_CLIENT_TEST_LOCK.lock().await;
        let (endpoint, server) = serve_proxy(b"").await;
        let request = common_utils::request::Request::new(
            common_utils::request::Method::Get,
            "http://connector.example/",
        );
        let error = super::super::send_request(&proxy(endpoint), request, Some(1))
            .await
            .expect_err("read timeout");
        assert!(matches!(
            error.current_context(),
            hyperswitch_interfaces::errors::HttpClientError::RequestTimeoutReceived
        ));
        assert!(error.downcast_ref::<ConfirmedProxyFailure>().is_none());
        server.abort();
    }

    #[tokio::test]
    async fn connector_read_timeout_does_not_count_when_proxy_is_reachable() {
        let (endpoint, server) = serve_proxy(b"").await;
        let error = client(&endpoint)
            .get("http://connector.example/")
            .timeout(Duration::from_millis(100))
            .send()
            .await
            .expect_err("read timeout");
        assert!(error.is_timeout());
        assert_eq!(failure_reason(&error, &proxy(endpoint)).await, None);
        server.abort();
    }

    #[tokio::test]
    async fn request_timeout_requires_an_unreachable_proxy_to_count() {
        let (endpoint, server) = serve_proxy(b"").await;
        let error = client(&endpoint)
            .get("http://connector.example/")
            .timeout(Duration::from_millis(100))
            .send()
            .await
            .expect_err("request deadline");
        assert!(error.is_timeout());
        assert!(!error.is_connect());
        server.abort();
        let _ = server.await;
        assert_eq!(
            failure_reason(&error, &proxy(endpoint)).await,
            Some("proxy_connection_failure")
        );
    }

    #[tokio::test]
    async fn upstream_connect_failure_does_not_count_when_proxy_is_reachable() {
        let (endpoint, server) =
            serve_proxy(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n").await;
        let error = client(&endpoint)
            .get("https://connector.example/")
            .send()
            .await
            .expect_err("upstream CONNECT failed");
        assert!(error.is_connect());
        assert_eq!(failure_reason(&error, &proxy(endpoint)).await, None);
        server.abort();
    }

    #[tokio::test]
    async fn connect_proxy_authentication_failure_is_identified() {
        let (endpoint, server) =
            serve_proxy(b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n")
                .await;
        let error = client(&endpoint)
            .get("https://connector.example/")
            .send()
            .await
            .expect_err("proxy authentication required");
        assert_eq!(
            failure_reason(&error, &proxy(endpoint)).await,
            Some("proxy_authentication_required")
        );
        server.abort();
    }

    #[tokio::test]
    async fn truncated_connector_body_does_not_count() {
        let (endpoint, server) =
            serve_proxy(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nshort").await;
        let response = client(&endpoint)
            .get("http://connector.example/")
            .send()
            .await
            .expect("HTTP response");
        let error = response.bytes().await.expect_err("truncated response body");
        assert_eq!(failure_reason(&error, &proxy(endpoint)).await, None);
        server.abort();
    }

    #[tokio::test]
    async fn bypassed_connector_connection_failure_does_not_count() {
        let endpoint = unused_endpoint().await;
        let destination = unused_endpoint().await;
        let client = reqwest::Client::builder()
            .proxy(
                reqwest::Proxy::all(&endpoint)
                    .expect("proxy")
                    .no_proxy(reqwest::NoProxy::from_string("127.0.0.1")),
            )
            .build()
            .expect("client");
        let error = client
            .get(destination)
            .send()
            .await
            .expect_err("connector is unreachable");
        let mut config = proxy(endpoint);
        config.bypass_proxy_hosts = Some("127.0.0.1".to_owned());
        assert_eq!(failure_reason(&error, &config).await, None);
    }

    #[tokio::test]
    async fn failure_without_a_configured_proxy_does_not_count() {
        let destination = unused_endpoint().await;
        let error = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client")
            .get(destination)
            .send()
            .await
            .expect_err("connector is unreachable");
        assert_eq!(failure_reason(&error, &Proxy::default()).await, None);
    }
}
