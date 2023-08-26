use std::time::Duration;

use error_stack::{IntoReport, ResultExt};
use http::{HeaderValue, Method};
use masking::PeekInterface;
use once_cell::sync::OnceCell;
use reqwest::{multipart::Form, IntoUrl};

use super::request::Maskable;
use crate::{
    configs::settings::{Locker, Proxy},
    core::{
        errors::{ApiClientError, CustomResult},
        payments,
    },
};

// static NON_PROXIED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static PROXIED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

fn get_client_builder(
    proxy_config: &Proxy,
    proxy_exluded_urls: Vec<String>,
) -> CustomResult<reqwest::ClientBuilder, ApiClientError> {
    let mut client_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .pool_idle_timeout(std::time::Duration::from_secs(
            proxy_config
                .idle_pool_connection_timeout
                .unwrap_or_default(),
        ));

    // if should_bypass_proxy {
    //     return Ok(client_builder);
    // }

    let no_proxy_urls = proxy_exluded_urls.join(", ");

    // Proxy all HTTPS traffic through the configured HTTPS proxy
    if let Some(url) = proxy_config.https_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::https(url)
                .into_report()
                .change_context(ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTPS proxy configuration error")?
                .no_proxy(reqwest::NoProxy::from_string(&no_proxy_urls)),
        );
    }

    // Proxy all HTTP traffic through the configured HTTP proxy
    if let Some(url) = proxy_config.http_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::http(url)
                .into_report()
                .change_context(ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTP proxy configuration error")?
                .no_proxy(reqwest::NoProxy::from_string(&no_proxy_urls)),
        );
    }

    Ok(client_builder)
}

fn get_base_client(
    proxy_config: &Proxy,
    proxy_exluded_urls: Vec<String>,
) -> CustomResult<reqwest::Client, ApiClientError> {
    Ok(PROXIED_CLIENT
        .get_or_try_init(|| {
            get_client_builder(proxy_config, proxy_exluded_urls)?
                .build()
                .into_report()
                .change_context(ApiClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct base client")
        })?
        .clone()
    )
}

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
pub(super) fn create_client(
    proxy_config: &Proxy,
    proxy_exluded_urls: Vec<String>,
    client_certificate: Option<String>,
    client_certificate_key: Option<String>,
) -> CustomResult<reqwest::Client, ApiClientError> {
    match (client_certificate, client_certificate_key) {
        (Some(encoded_certificate), Some(encoded_certificate_key)) => {
            let client_builder = get_client_builder(proxy_config, proxy_exluded_urls)?;

            let identity = payments::helpers::create_identity_from_certificate_and_key(
                encoded_certificate,
                encoded_certificate_key,
            )?;

            client_builder
                .identity(identity)
                .build()
                .into_report()
                .change_context(ApiClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct client with certificate and certificate key")
        }
        _ => get_base_client(proxy_config, proxy_exluded_urls),
    }
}

pub(super) fn proxy_bypass_urls(locker: &Locker) -> Vec<String> {
    let locker_host = locker.host.to_owned();
    let basilisk_host = locker.basilisk_host.to_owned();
    vec![
        format!("{locker_host}/cards/add"),
        format!("{locker_host}/cards/retrieve"),
        format!("{locker_host}/cards/delete"),
        format!("{locker_host}/card/addCard"),
        format!("{locker_host}/card/getCard"),
        format!("{locker_host}/card/deleteCard"),
        format!("{basilisk_host}/tokenize"),
        format!("{basilisk_host}/tokenize/get"),
        format!("{basilisk_host}/tokenize/delete"),
        format!("{basilisk_host}/tokenize/delete/token"),
    ]
}

pub trait RequestBuilder: Send + Sync {
    fn json(&mut self, body: serde_json::Value);
    fn url_encoded_form(&mut self, body: serde_json::Value);
    fn timeout(&mut self, timeout: Duration);
    fn multipart(&mut self, form: Form);
    fn header(&mut self, key: String, value: Maskable<String>) -> CustomResult<(), ApiClientError>;
    fn send(
        self,
    ) -> CustomResult<
        Box<
            (dyn core::future::Future<Output = Result<reqwest::Response, reqwest::Error>>
                 + 'static),
        >,
        ApiClientError,
    >;
}

pub trait ApiClient
where
    Self: Sized + Send + Sync,
{
    fn new(
        proxy_config: Proxy,
        whitelisted_urls: Vec<String>,
    ) -> CustomResult<Self, ApiClientError>;
    fn request<U: IntoUrl>(
        &self,
        method: Method,
        url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;
    fn request_with_certificate(
        &self,
        method: Method,
        url: String,
        certificate: Option<String>,
        certificate_key: Option<String>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;
}

#[derive(Clone)]
pub struct ProxyClient {
    proxy_client: reqwest::Client,
    // non_proxy_client: reqwest::Client,
    whitelisted_urls: Vec<String>,
}

impl ProxyClient {
    fn get_reqwest_client(
        &self,
        base_url: String,
        client_certificate: Option<String>,
        client_certificate_key: Option<String>,
    ) -> CustomResult<reqwest::Client, ApiClientError> {
        match (client_certificate, client_certificate_key) {
            (Some(certificate), Some(certificate_key)) => {
                let client_builder =
                    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
                let identity = payments::helpers::create_identity_from_certificate_and_key(
                    certificate,
                    certificate_key,
                )?;
                Ok(client_builder
                    .identity(identity)
                    .build()
                    .into_report()
                    .change_context(ApiClientError::ClientConstructionFailed)
                    .attach_printable(
                        "Failed to construct client with certificate and certificate key",
                    )?)
            }
            (_, _) => Ok(self.proxy_client.clone())
        }
    }
}

pub struct RouterRequestBuilder {
    // Using option here to get around the reinitialization problem
    // request builder follows a chain pattern where the value is consumed and a newer requestbuilder is returned
    // Since for this brief period of time between the value being consumed & newer request builder
    // since requestbuilder does not allow moving the value
    // leaves our struct in an inconsistent state, we are using option to get around rust semantics
    inner: Option<reqwest::RequestBuilder>,
}

impl RequestBuilder for RouterRequestBuilder {
    fn json(&mut self, body: serde_json::Value) {
        self.inner = self.inner.take().map(|r| r.json(&body));
    }
    fn url_encoded_form(&mut self, body: serde_json::Value) {
        self.inner = self.inner.take().map(|r| r.form(&body));
    }

    fn timeout(&mut self, timeout: Duration) {
        self.inner = self.inner.take().map(|r| r.timeout(timeout));
    }

    fn multipart(&mut self, form: Form) {
        self.inner = self.inner.take().map(|r| r.multipart(form));
    }

    fn header(&mut self, key: String, value: Maskable<String>) -> CustomResult<(), ApiClientError> {
        let header_value = match value {
            Maskable::Masked(hvalue) => HeaderValue::from_str(hvalue.peek()).map(|mut h| {
                h.set_sensitive(true);
                h
            }),
            Maskable::Normal(hvalue) => HeaderValue::from_str(&hvalue),
        }
        .into_report()
        .change_context(ApiClientError::HeaderMapConstructionFailed)?;

        self.inner = self.inner.take().map(|r| r.header(key, header_value));
        Ok(())
    }

    fn send(
        self,
    ) -> CustomResult<
        Box<
            (dyn core::future::Future<Output = Result<reqwest::Response, reqwest::Error>>
                 + 'static),
        >,
        ApiClientError,
    > {
        Ok(Box::new(
            self.inner.ok_or(ApiClientError::UnexpectedState)?.send(),
        ))
    }
}

// TODO: remove this when integrating this trait
#[allow(dead_code)]
impl ApiClient for ProxyClient {
    fn new(
        proxy_config: Proxy,
        whitelisted_urls: Vec<String>,
    ) -> CustomResult<Self, ApiClientError> {
        // let non_proxy_client = reqwest::Client::builder()
        //     .redirect(reqwest::redirect::Policy::none())
        //     .build()
        //     .into_report()
        //     .change_context(ApiClientError::ClientConstructionFailed)?;
        let no_proxy_urls = whitelisted_urls.join(", ");

        let mut proxy_builder =
            reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());

        if let Some(url) = proxy_config.https_url.as_ref() {
            proxy_builder = proxy_builder.proxy(
                reqwest::Proxy::https(url)
                    .into_report()
                    .change_context(ApiClientError::InvalidProxyConfiguration)?
                    .no_proxy(reqwest::NoProxy::from_string(&no_proxy_urls)),
            );
        }

        if let Some(url) = proxy_config.http_url.as_ref() {
            proxy_builder = proxy_builder.proxy(
                reqwest::Proxy::http(url)
                    .into_report()
                    .change_context(ApiClientError::InvalidProxyConfiguration)?
                    .no_proxy(reqwest::NoProxy::from_string(&no_proxy_urls)),
            );
        }

        let proxy_client = proxy_builder
            .build()
            .into_report()
            .change_context(ApiClientError::InvalidProxyConfiguration)?;
        Ok(Self {
            proxy_client,
            // non_proxy_client,
            whitelisted_urls,
        })
    }

    fn request<U: IntoUrl>(
        &self,
        method: Method,
        url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        self.request_with_certificate(method, url, None, None)
    }

    fn request_with_certificate(
        &self,
        method: Method,
        url: String,
        certificate: Option<String>,
        certificate_key: Option<String>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        let client_builder = self
            .get_reqwest_client(url.clone(), certificate, certificate_key)
            .change_context(ApiClientError::ClientConstructionFailed)?;
        Ok(Box::new(RouterRequestBuilder {
            inner: Some(client_builder.request(method, url)),
        }))
    }
}
