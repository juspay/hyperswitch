use std::time::Duration;

use base64::Engine;
use error_stack::ResultExt;
use http::{HeaderValue, Method};
use masking::{ExposeInterface, PeekInterface};
use once_cell::sync::OnceCell;
use reqwest::multipart::Form;
use router_env::tracing_actix_web::RequestId;

use super::{request::Maskable, Request};
use crate::{
    configs::settings::Proxy,
    consts::BASE64_ENGINE,
    core::errors::{ApiClientError, CustomResult},
    routes::SessionState,
};

static DEFAULT_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

fn get_client_builder(
    proxy_config: &Proxy,
) -> CustomResult<reqwest::ClientBuilder, ApiClientError> {
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
                .change_context(ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTPS proxy configuration error")?
                .no_proxy(proxy_exclusion_config.clone()),
        );
    }

    // Proxy all HTTP traffic through the configured HTTP proxy
    if let Some(url) = proxy_config.http_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::http(url)
                .change_context(ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTP proxy configuration error")?
                .no_proxy(proxy_exclusion_config),
        );
    }

    Ok(client_builder)
}

fn get_base_client(proxy_config: &Proxy) -> CustomResult<reqwest::Client, ApiClientError> {
    Ok(DEFAULT_CLIENT
        .get_or_try_init(|| {
            get_client_builder(proxy_config)?
                .build()
                .change_context(ApiClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct base client")
        })?
        .clone())
}

// We may need to use outbound proxy to connect to external world.
// Precedence will be the environment variables, followed by the config.
pub fn create_client(
    proxy_config: &Proxy,
    client_certificate: Option<masking::Secret<String>>,
    client_certificate_key: Option<masking::Secret<String>>,
) -> CustomResult<reqwest::Client, ApiClientError> {
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
                .change_context(ApiClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct client with certificate and certificate key")
        }
        _ => get_base_client(proxy_config),
    }
}

pub fn create_identity_from_certificate_and_key(
    encoded_certificate: masking::Secret<String>,
    encoded_certificate_key: masking::Secret<String>,
) -> Result<reqwest::Identity, error_stack::Report<ApiClientError>> {
    let decoded_certificate = BASE64_ENGINE
        .decode(encoded_certificate.expose())
        .change_context(ApiClientError::CertificateDecodeFailed)?;

    let decoded_certificate_key = BASE64_ENGINE
        .decode(encoded_certificate_key.expose())
        .change_context(ApiClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .change_context(ApiClientError::CertificateDecodeFailed)?;

    let certificate_key = String::from_utf8(decoded_certificate_key)
        .change_context(ApiClientError::CertificateDecodeFailed)?;

    let key_chain = format!("{}{}", certificate_key, certificate);
    reqwest::Identity::from_pem(key_chain.as_bytes())
        .change_context(ApiClientError::CertificateDecodeFailed)
}

pub fn create_certificate(
    encoded_certificate: masking::Secret<String>,
) -> Result<Vec<reqwest::Certificate>, error_stack::Report<ApiClientError>> {
    let decoded_certificate = BASE64_ENGINE
        .decode(encoded_certificate.expose())
        .change_context(ApiClientError::CertificateDecodeFailed)?;

    let certificate = String::from_utf8(decoded_certificate)
        .change_context(ApiClientError::CertificateDecodeFailed)?;
    reqwest::Certificate::from_pem_bundle(certificate.as_bytes())
        .change_context(ApiClientError::CertificateDecodeFailed)
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

#[async_trait::async_trait]
pub trait ApiClient: dyn_clone::DynClone
where
    Self: Send + Sync,
{
    fn request(
        &self,
        method: Method,
        url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;

    fn request_with_certificate(
        &self,
        method: Method,
        url: String,
        certificate: Option<masking::Secret<String>>,
        certificate_key: Option<masking::Secret<String>>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;

    async fn send_request(
        &self,
        state: &SessionState,
        request: Request,
        option_timeout_secs: Option<u64>,
        forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError>;

    fn add_request_id(&mut self, request_id: RequestId);

    fn get_request_id(&self) -> Option<String>;

    fn add_flow_name(&mut self, flow_name: String);
}

dyn_clone::clone_trait_object!(ApiClient);

#[derive(Clone)]
pub struct ProxyClient {
    proxy_config: Proxy,
    client: reqwest::Client,
    request_id: Option<String>,
}

impl ProxyClient {
    pub fn new(proxy_config: &Proxy) -> CustomResult<Self, ApiClientError> {
        let client = get_client_builder(proxy_config)?
            .build()
            .change_context(ApiClientError::InvalidProxyConfiguration)?;
        Ok(Self {
            proxy_config: proxy_config.clone(),
            client,
            request_id: None,
        })
    }

    pub fn get_reqwest_client(
        &self,
        client_certificate: Option<masking::Secret<String>>,
        client_certificate_key: Option<masking::Secret<String>>,
    ) -> CustomResult<reqwest::Client, ApiClientError> {
        match (client_certificate, client_certificate_key) {
            (Some(certificate), Some(certificate_key)) => {
                let client_builder = get_client_builder(&self.proxy_config)?;
                let identity =
                    create_identity_from_certificate_and_key(certificate, certificate_key)?;
                Ok(client_builder
                    .identity(identity)
                    .build()
                    .change_context(ApiClientError::ClientConstructionFailed)
                    .attach_printable(
                        "Failed to construct client with certificate and certificate key",
                    )?)
            }
            (_, _) => Ok(self.client.clone()),
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

#[async_trait::async_trait]
impl ApiClient for ProxyClient {
    fn request(
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
        certificate: Option<masking::Secret<String>>,
        certificate_key: Option<masking::Secret<String>>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        let client_builder = self
            .get_reqwest_client(certificate, certificate_key)
            .change_context(ApiClientError::ClientConstructionFailed)?;
        Ok(Box::new(RouterRequestBuilder {
            inner: Some(client_builder.request(method, url)),
        }))
    }
    async fn send_request(
        &self,
        state: &SessionState,
        request: Request,
        option_timeout_secs: Option<u64>,
        _forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError> {
        crate::services::send_request(state, request, option_timeout_secs).await
    }

    fn add_request_id(&mut self, request_id: RequestId) {
        self.request_id
            .replace(request_id.as_hyphenated().to_string());
    }

    fn get_request_id(&self) -> Option<String> {
        self.request_id.clone()
    }

    fn add_flow_name(&mut self, _flow_name: String) {}
}

/// Api client for testing sending request
#[derive(Clone)]
pub struct MockApiClient;

#[async_trait::async_trait]
impl ApiClient for MockApiClient {
    fn request(
        &self,
        _method: Method,
        _url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        // [#2066]: Add Mock implementation for ApiClient
        Err(ApiClientError::UnexpectedState.into())
    }

    fn request_with_certificate(
        &self,
        _method: Method,
        _url: String,
        _certificate: Option<masking::Secret<String>>,
        _certificate_key: Option<masking::Secret<String>>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        // [#2066]: Add Mock implementation for ApiClient
        Err(ApiClientError::UnexpectedState.into())
    }

    async fn send_request(
        &self,
        _state: &SessionState,
        _request: Request,
        _option_timeout_secs: Option<u64>,
        _forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError> {
        // [#2066]: Add Mock implementation for ApiClient
        Err(ApiClientError::UnexpectedState.into())
    }

    fn add_request_id(&mut self, _request_id: RequestId) {
        // [#2066]: Add Mock implementation for ApiClient
    }

    fn get_request_id(&self) -> Option<String> {
        // [#2066]: Add Mock implementation for ApiClient
        None
    }

    fn add_flow_name(&mut self, _flow_name: String) {}
}
