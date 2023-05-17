use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use once_cell::sync::OnceCell;

use crate::{
    configs::settings::{Locker, Proxy},
    consts,
    core::errors::{self, CustomResult},
};

static NON_PROXIED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static PROXIED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

fn get_client_builder(
    proxy_config: &Proxy,
    should_bypass_proxy: bool,
) -> CustomResult<reqwest::ClientBuilder, errors::ApiClientError> {
    let mut client_builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());

    if should_bypass_proxy {
        return Ok(client_builder);
    }

    // Proxy all HTTPS traffic through the configured HTTPS proxy
    if let Some(url) = proxy_config.https_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::https(url)
                .into_report()
                .change_context(errors::ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTPS proxy configuration error")?,
        );
    }

    // Proxy all HTTP traffic through the configured HTTP proxy
    if let Some(url) = proxy_config.http_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::http(url)
                .into_report()
                .change_context(errors::ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTP proxy configuration error")?,
        );
    }

    Ok(client_builder)
}

fn get_base_client(
    proxy_config: &Proxy,
    should_bypass_proxy: bool,
) -> CustomResult<reqwest::Client, errors::ApiClientError> {
    Ok(if should_bypass_proxy
        || (proxy_config.http_url.is_none() && proxy_config.https_url.is_none())
    {
        &NON_PROXIED_CLIENT
    } else {
        &PROXIED_CLIENT
    }
    .get_or_try_init(|| {
        get_client_builder(proxy_config, should_bypass_proxy)?
            .build()
            .into_report()
            .change_context(errors::ApiClientError::ClientConstructionFailed)
            .attach_printable("Failed to construct base client")
    })?
    .clone())
}

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
pub(super) fn create_client(
    proxy_config: &Proxy,
    should_bypass_proxy: bool,
    client_certificate: Option<String>,
    client_certificate_key: Option<String>,
) -> CustomResult<reqwest::Client, errors::ApiClientError> {
    match (client_certificate, client_certificate_key) {
        (Some(encoded_cert), Some(encoded_cert_key)) => {
            let client_builder = get_client_builder(proxy_config, should_bypass_proxy)?;

            let decoded_cert = consts::BASE64_ENGINE
                .decode(encoded_cert)
                .into_report()
                .change_context(errors::ApiClientError::CertificateDecodeFailed)?;
            let decoded_cert_key = consts::BASE64_ENGINE
                .decode(encoded_cert_key)
                .into_report()
                .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

            let certificate = String::from_utf8(decoded_cert)
                .into_report()
                .change_context(errors::ApiClientError::CertificateDecodeFailed)?;
            let certificate_key = String::from_utf8(decoded_cert_key)
                .into_report()
                .change_context(errors::ApiClientError::CertificateDecodeFailed)?;
            let identity = reqwest::Identity::from_pkcs8_pem(
                certificate.as_bytes(),
                certificate_key.as_bytes(),
            )
            .into_report()
            .change_context(errors::ApiClientError::CertificateDecodeFailed)?;

            client_builder
                .identity(identity)
                .build()
                .into_report()
                .change_context(errors::ApiClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct client with certificate and certificate key")
        }
        _ => get_base_client(proxy_config, should_bypass_proxy),
    }
}

pub(super) fn should_bypass_proxy(locker: &Locker, url: &str) -> bool {
    let locker_host = locker.host.to_owned();
    let basilisk_host = locker.basilisk_host.to_owned();
    let bypass_list = vec![
        locker_host,
        basilisk_host,
        "http://localhost:8080/dummy-connector".to_string(),
    ];
    for bypass_url in bypass_list.iter() {
        if url.starts_with(bypass_url) {
            return true;
        }
    }
    false
}
