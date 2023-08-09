use std::time::Duration;

use error_stack::{IntoReport, ResultExt};
use reqwest::multipart::Form;
use http::{HeaderName, HeaderValue, Method};
use masking::PeekInterface;
use once_cell::sync::OnceCell;
use reqwest::IntoUrl;

use super::request::Maskable;
use crate::{
    configs::settings::{Locker, Proxy},
    core::{
        errors::{self, CustomResult},
        payments,
    },
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
        (Some(encoded_certificate), Some(encoded_certificate_key)) => {
            let client_builder = get_client_builder(proxy_config, should_bypass_proxy)?;

            let identity = payments::helpers::create_identity_from_certificate_and_key(
                encoded_certificate,
                encoded_certificate_key,
            )?;

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
    fn json(&mut self, body: serde_json::Value) -> CustomResult<(), &'static str>;
    fn url_encoded_form(&mut self, body: serde_json::Value) -> CustomResult<(), &'static str>;
    fn timeout(&mut self, timeout: Duration);
    fn multipart(&mut self, form: Form);
    fn header(&mut self, key: String, value: Maskable<String>) -> CustomResult<(), &'static str>;
    fn send(
        self,
    ) -> Box<dyn core::future::Future<Output = Result<reqwest::Response, reqwest::Error>>>;
}

pub trait ApiClient
where
    Self: Sized + Send + Sync,
{
    fn new(proxy_config: Proxy, whitelisted_urls: Vec<String>) -> CustomResult<Self, &'static str>;
    fn request<U: IntoUrl>(&self, method: Method, url: U) -> Box<dyn RequestBuilder>;
}

#[derive(Clone)]
pub struct ProxyClient {
    proxy_client: reqwest::Client,
    non_proxy_client: reqwest::Client,
    whitelisted_urls: Vec<String>,
}

impl ProxyClient {
    fn get_reqwest_client(
        &self,
        base_url: String,
        client_certificate: Option<String>,
        client_certificate_key: Option<String>,
    ) -> reqwest::Client {
        // Fix this shit as well
        if self.whitelisted_urls.contains(&base_url) {
            self.non_proxy_client.clone()
        } else {
            self.proxy_client.clone()
        }
    }
}

#[derive(Clone)]
pub struct RouterRequestBuilder {
    inner: reqwest::Request,
    client: ProxyClient,
}

impl RequestBuilder for RouterRequestBuilder {
    fn json(&mut self, body: serde_json::Value) -> CustomResult<(), &'static str> {
        let body_bytes = serde_json::to_vec(&body).map_err(ToString::to_string)?;
        self.inner
            .body_mut()
            .replace(reqwest::Body::from(body_bytes));
        Ok(())
    }
    fn url_encoded_form(&mut self, body: serde_json::Value) -> CustomResult<(), &'static str> {
        let url_encoded_payload = serde_urlencoded::to_string(&body).map_err(ToString::to_string)?;
        self.inner.body_mut().replace(reqwest::Body::from(url_encoded_payload));
        Ok(())

    }

    fn timeout(&mut self, timeout: Duration) {
        self.inner.timeout_mut().replace(timeout);
    }

    fn multipart(&mut self, form: Form) {
        self.inner = self.inner.multipart(form);
    }

    fn header(&mut self, key: String, value: Maskable<String>) -> CustomResult<(), &'static str> {
        let header_value = match value {
            Maskable::Masked(hvalue) => {
                let mut header =
                    HeaderValue::from_str(hvalue.peek())?;
                header.set_sensitive(true);
                Ok(header)
            }
            Maskable::Normal(hvalue) => HeaderValue::from_str(&hvalue),
        }.into_report().change_context("header creation failed")?;
        let header_key = HeaderName::try_from(key);
        self.inner.headers_mut().append(header_key, header_value);
        Ok(())
    }

    fn send(
        self,
    ) -> Box<dyn core::future::Future<Output = Result<reqwest::Response, reqwest::Error>>> {
        // Add client selection logic here
        Box::new(self.client.proxy_client.execute(self.inner))
    }
}

impl ApiClient for ProxyClient {
    fn new(proxy_config: Proxy, whitelisted_urls: Vec<String>) -> CustomResult<Self, &'static str> {
        let non_proxy_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build().into_report().change_context("NON-Proxy client building failed")?;

        let mut proxy_builder =
            reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());

        if let Some(url) = proxy_config.https_url.as_ref() {
            proxy_builder =
                proxy_builder.proxy(reqwest::Proxy::https(url).into_report().change_context("Proxy HTTP URL is invalid"))?;
        }

        if let Some(url) = proxy_config.http_url.as_ref() {
            proxy_builder =
                proxy_builder.proxy(reqwest::Proxy::http(url).into_report().change_context("Proxy HTTP URL is invalid"))?;
        }

        let proxy_client = proxy_builder.build().into_report().change_context("Proxy client building failed")?;

        Ok(Self {
            proxy_client,
            non_proxy_client,
            whitelisted_urls,
        })
    }

    fn request<U: IntoUrl>(&self, method: Method, url: U) -> Box<dyn RequestBuilder> {
        Box::new(RouterRequestBuilder {
            inner: self.proxy_client.request(method, url).build().map_err(|er| format!("{er:?}")).unwrap(),
            client: self.clone(),
        })
    }
}
