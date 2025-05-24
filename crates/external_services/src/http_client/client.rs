use std::time::Duration;

use base64::Engine;
use common_utils::consts::BASE64_ENGINE;
pub use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use masking::ExposeInterface;
use router_env::logger;

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
#[allow(missing_docs)]
pub fn create_client(
    proxy_config: &Proxy,
    client_certificate: Option<masking::Secret<String>>,
    client_certificate_key: Option<masking::Secret<String>>,
    ca_certificate: Option<masking::Secret<String>>,
) -> CustomResult<reqwest::Client, HttpClientError> {
    let mut client_builder = get_client_builder(proxy_config)?;

    // If CA certificate is provided for server authentication only (one-way TLS)
    if let Some(ca_pem) = ca_certificate {
        let pem = ca_pem.expose().replace("\\r\\n", "\n"); // Fix escaped newlines
        let cert = reqwest::Certificate::from_pem(pem.as_bytes())
            .change_context(HttpClientError::ClientConstructionFailed)
            .attach_printable("Failed to parse CA certificate PEM block")?;
        client_builder = client_builder.add_root_certificate(cert);
    }

    // If client cert and key are provided (for mutual TLS)
    match (client_certificate, client_certificate_key) {
        (Some(cert), Some(key)) => {
            let identity = create_identity_from_certificate_and_key(cert.clone(), key)
                .change_context(HttpClientError::ClientConstructionFailed)
                .attach_printable("Failed to create identity from client cert and key")?;
            let certificate_list = create_certificate(cert)
                .change_context(HttpClientError::ClientConstructionFailed)
                .attach_printable("Failed to parse certificate list")?;
            for cert in certificate_list {
                client_builder = client_builder.add_root_certificate(cert);
            }
            client_builder = client_builder.identity(identity);
        }
        (Some(_), None) | (None, Some(_)) => {
            logger::warn!("Incomplete mTLS setup: client certificate or key missing. Skipping mTLS configuration.");
        }
        _ => {}
    }

    client_builder
        .use_rustls_tls()
        .build()
        .change_context(HttpClientError::ClientConstructionFailed)
        .attach_printable("Failed to construct HTTP client")
}

#[allow(missing_docs)]
pub fn get_client_builder(
    proxy_config: &Proxy,
) -> CustomResult<reqwest::ClientBuilder, HttpClientError> {
    let mut client_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .pool_idle_timeout(Duration::from_secs(
            proxy_config
                .idle_pool_connection_timeout
                .unwrap_or_default(),
        ));

    let proxy_exclusion_config =
        reqwest::NoProxy::from_string(&proxy_config.bypass_proxy_hosts.clone().unwrap_or_default());

    // Proxy all HTTPS traffic through the configured HTTPS proxy
    if let Some(url) = proxy_config.https_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::https(url)
                .change_context(HttpClientError::InvalidProxyConfiguration)
                .attach_printable("HTTPS proxy configuration error")?
                .no_proxy(proxy_exclusion_config.clone()),
        );
    }

    // Proxy all HTTP traffic through the configured HTTP proxy
    if let Some(url) = proxy_config.http_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::http(url)
                .change_context(HttpClientError::InvalidProxyConfiguration)
                .attach_printable("HTTP proxy configuration error")?
                .no_proxy(proxy_exclusion_config),
        );
    }

    Ok(client_builder)
}

#[allow(missing_docs)]
pub fn create_identity_from_certificate_and_key(
    encoded_certificate: masking::Secret<String>,
    encoded_certificate_key: masking::Secret<String>,
) -> Result<reqwest::Identity, error_stack::Report<HttpClientError>> {
    let decoded_certificate = BASE64_ENGINE
        .decode(encoded_certificate.expose())
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let decoded_certificate_key = BASE64_ENGINE
        .decode(encoded_certificate_key.expose())
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let certificate_key = String::from_utf8(decoded_certificate_key)
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let key_chain = format!("{}{}", certificate_key, certificate);
    reqwest::Identity::from_pem(key_chain.as_bytes())
        .change_context(HttpClientError::CertificateDecodeFailed)
}

#[allow(missing_docs)]
pub fn create_certificate(
    encoded_certificate: masking::Secret<String>,
) -> Result<Vec<reqwest::Certificate>, error_stack::Report<HttpClientError>> {
    let decoded_certificate = BASE64_ENGINE
        .decode(encoded_certificate.expose())
        .change_context(HttpClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .change_context(HttpClientError::CertificateDecodeFailed)?;
    reqwest::Certificate::from_pem_bundle(certificate.as_bytes())
        .change_context(HttpClientError::CertificateDecodeFailed)
}
