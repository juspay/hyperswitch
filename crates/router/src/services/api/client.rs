use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use once_cell::sync::OnceCell;

use crate::{
    configs::settings::{Locker, Proxy},
    consts,
    core::errors::{self, CustomResult},
};

static PLAIN_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static HTTPS_PROXY_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static HTTP_PROXY_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

enum ProxyType {
    Http,
    Https,
}

impl ProxyType {
    fn get_proxy_url(&self, proxy: &Proxy) -> Option<String> {
        match self {
            Self::Http => proxy.http_url.clone(),
            Self::Https => proxy.https_url.clone(),
        }
    }
}

fn create_base_client(
    proxy: Option<(ProxyType, String)>,
) -> CustomResult<reqwest::Client, errors::ApiClientError> {
    Ok(match proxy {
        None => &PLAIN_CLIENT,
        Some((ProxyType::Http, _)) => &HTTP_PROXY_CLIENT,
        Some((ProxyType::Https, _)) => &HTTPS_PROXY_CLIENT,
    }
    .get_or_try_init(|| {
        let mut cb = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
        cb = match proxy {
            None => cb,
            Some((proxy_type, url)) => cb.proxy(
                match proxy_type {
                    ProxyType::Http => reqwest::Proxy::http(url),
                    ProxyType::Https => reqwest::Proxy::https(url),
                }
                .into_report()
                .change_context(errors::ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTP proxy configuration error")?,
            ),
        };
        cb.build()
            .into_report()
            .change_context(errors::ApiClientError::ClientConstructionFailed)
            .attach_printable("Error with client library")
    })?
    .clone())
}

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
pub(super) fn create_client(
    proxy: &Proxy,
    should_bypass_proxy: bool,
    client_certificate: Option<String>,
    client_certificate_key: Option<String>,
) -> CustomResult<reqwest::Client, errors::ApiClientError> {
    if client_certificate.is_none() && client_certificate_key.is_none() {
        return match should_bypass_proxy {
            true => create_base_client(None),
            false => create_base_client(
                ProxyType::Https
                    .get_proxy_url(proxy)
                    .map(|url| (ProxyType::Https, url))
                    .or_else(|| {
                        ProxyType::Http
                            .get_proxy_url(proxy)
                            .map(|url| (ProxyType::Http, url))
                    }),
            ),
        };
    }
    let mut client_builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());

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

    client_builder = match (client_certificate, client_certificate_key) {
        (Some(encoded_cert), Some(encoded_cert_key)) => {
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

            client_builder.identity(identity)
        }
        _ => client_builder,
    };

    client_builder
        .build()
        .into_report()
        .change_context(errors::ApiClientError::ClientConstructionFailed)
        .attach_printable_lazy(|| "Error with client library")
}

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
