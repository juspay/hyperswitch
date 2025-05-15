use std::time::Duration;

use base64::Engine;
use common_utils::consts::BASE64_ENGINE;
pub use common_utils::errors::CustomResult;
use error_stack::ResultExt;
use hyperswitch_interfaces::{errors::HttpClientError, types::Proxy};
use masking::ExposeInterface;
use once_cell::sync::OnceCell;

static DEFAULT_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
#[allow(missing_docs)]
pub fn create_client(
    proxy_config: &Proxy,
    client_certificate: Option<masking::Secret<String>>,
    client_certificate_key: Option<masking::Secret<String>>,
) -> CustomResult<reqwest::Client, HttpClientError> {
    match (client_certificate, client_certificate_key) {
        (Some(encoded_certificate), Some(encoded_certificate_key)) => {
            let client_builder = get_client_builder(proxy_config)?;

            let identity = create_identity_from_certificate_and_key(
                encoded_certificate.clone(),
                encoded_certificate_key,
            )?;
            let certificate_list = create_certificate(encoded_certificate)?;
            let client_builder = certificate_list
                .into_iter()
                .fold(client_builder, |client_builder, certificate| {
                    client_builder.add_root_certificate(certificate)
                });
            client_builder
                .identity(identity)
                .use_rustls_tls()
                .build()
                .change_context(HttpClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct client with certificate and certificate key")
        }
        _ => get_base_client(proxy_config),
    }
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

fn get_base_client(proxy_config: &Proxy) -> CustomResult<reqwest::Client, HttpClientError> {
    Ok(DEFAULT_CLIENT
        .get_or_try_init(|| {
            get_client_builder(proxy_config)?
                .build()
                .change_context(HttpClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct base client")
        })?
        .clone())
}
