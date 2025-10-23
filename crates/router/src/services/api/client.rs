use std::time::Duration;

use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
pub use external_services::http_client::{self, client};
use http::{HeaderValue, Method};
pub use hyperswitch_interfaces::{
    api_client::{ApiClient, ApiClientWrapper, RequestBuilder},
    types::Proxy,
};
use masking::PeekInterface;
use reqwest::multipart::Form;
use router_env::RequestId;

use super::{request::Maskable, Request};
use crate::core::errors::{ApiClientError, CustomResult};

#[derive(Clone)]
pub struct ProxyClient {
    proxy_config: Proxy,
    client: reqwest::Client,
    request_id: Option<RequestId>,
}

impl ProxyClient {
    pub fn new(proxy_config: &Proxy) -> CustomResult<Self, ApiClientError> {
        let client = client::get_client_builder(proxy_config)
            .switch()?
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
                let client_builder = client::get_client_builder(&self.proxy_config).switch()?;
                let identity =
                    client::create_identity_from_certificate_and_key(certificate, certificate_key)
                        .switch()?;
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
        Box<dyn core::future::Future<Output = Result<reqwest::Response, reqwest::Error>> + 'static>,
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
        api_client: &dyn ApiClientWrapper,
        request: Request,
        option_timeout_secs: Option<u64>,
        _forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError> {
        http_client::send_request(&api_client.get_proxy(), request, option_timeout_secs)
            .await
            .switch()
    }

    fn add_request_id(&mut self, request_id: RequestId) {
        self.request_id = Some(request_id);
    }

    fn get_request_id(&self) -> Option<RequestId> {
        self.request_id.clone()
    }

    fn get_request_id_str(&self) -> Option<String> {
        self.request_id.as_ref().map(|id| id.to_string())
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
        _state: &dyn ApiClientWrapper,
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

    fn get_request_id(&self) -> Option<RequestId> {
        // [#2066]: Add Mock implementation for ApiClient
        None
    }

    fn get_request_id_str(&self) -> Option<String> {
        // [#2066]: Add Mock implementation for ApiClient
        None
    }

    fn add_flow_name(&mut self, _flow_name: String) {}
}
