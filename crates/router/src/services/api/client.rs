use std::time::Duration;

use error_stack::{IntoReport, ResultExt};
use http::{HeaderValue, Method};
use masking::PeekInterface;
use once_cell::sync::OnceCell;
use reqwest::multipart::Form;
use router_env::tracing_actix_web::RequestId;

use super::{request::Maskable, Request};
use crate::{
    configs::settings::{Locker, Proxy},
    consts::LOCKER_HEALTH_CALL_PATH,
    core::{
        errors::{ApiClientError, CustomResult},
        payments,
    },
    routes::AppState,
};

static NON_PROXIED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static PROXIED_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

/// Returns a reqwest ClientBuilder with proxy configuration based on the provided Proxy and should_bypass_proxy flag.
fn get_client_builder(
    proxy_config: &Proxy,
    should_bypass_proxy: bool,
) -> CustomResult<reqwest::ClientBuilder, ApiClientError> {
    let mut client_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .pool_idle_timeout(std::time::Duration::from_secs(
            proxy_config
                .idle_pool_connection_timeout
                .unwrap_or_default(),
        ));

    if should_bypass_proxy {
        return Ok(client_builder);
    }

    // Proxy all HTTPS traffic through the configured HTTPS proxy
    if let Some(url) = proxy_config.https_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::https(url)
                .into_report()
                .change_context(ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTPS proxy configuration error")?,
        );
    }

    // Proxy all HTTP traffic through the configured HTTP proxy
    if let Some(url) = proxy_config.http_url.as_ref() {
        client_builder = client_builder.proxy(
            reqwest::Proxy::http(url)
                .into_report()
                .change_context(ApiClientError::InvalidProxyConfiguration)
                .attach_printable("HTTP proxy configuration error")?,
        );
    }

    Ok(client_builder)
}

/// Returns a base `reqwest::Client` based on the provided proxy configuration and a flag indicating whether to bypass the proxy. If the `should_bypass_proxy` flag is true or if both the HTTP and HTTPS URLs in the `proxy_config` are `None`, the method returns a non-proxied client. Otherwise, it returns a proxied client based on the provided proxy configuration. The method also handles the construction of the client and handles any potential errors by attaching context and printable information to the error message.
fn get_base_client(
    proxy_config: &Proxy,
    should_bypass_proxy: bool,
) -> CustomResult<reqwest::Client, ApiClientError> {
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
            .change_context(ApiClientError::ClientConstructionFailed)
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
) -> CustomResult<reqwest::Client, ApiClientError> {
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
                .change_context(ApiClientError::ClientConstructionFailed)
                .attach_printable("Failed to construct client with certificate and certificate key")
        }
        _ => get_base_client(proxy_config, should_bypass_proxy),
    }
}

/// This method takes a Locker reference and returns a Vec of strings containing URLs for proxy bypass. 
pub fn proxy_bypass_urls(locker: &Locker) -> Vec<String> {
    let locker_host = locker.host.to_owned();
    let locker_host_rs = locker.host_rs.to_owned();
    vec![
        format!("{locker_host}/cards/add"),
        format!("{locker_host}/cards/retrieve"),
        format!("{locker_host}/cards/delete"),
        format!("{locker_host_rs}/cards/add"),
        format!("{locker_host_rs}/cards/retrieve"),
        format!("{locker_host_rs}/cards/delete"),
        format!("{locker_host_rs}{}", LOCKER_HEALTH_CALL_PATH),
        format!("{locker_host}/card/addCard"),
        format!("{locker_host}/card/getCard"),
        format!("{locker_host}/card/deleteCard"),
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
        certificate: Option<String>,
        certificate_key: Option<String>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError>;

    async fn send_request(
        &self,
        state: &AppState,
        request: Request,
        option_timeout_secs: Option<u64>,
        forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError>;

    fn add_request_id(&mut self, request_id: RequestId);
    fn get_request_id(&self) -> Option<String>;
    fn add_merchant_id(&mut self, _merchant_id: Option<String>);
    fn add_flow_name(&mut self, flow_name: String);
}

dyn_clone::clone_trait_object!(ApiClient);

#[derive(Clone)]
pub struct ProxyClient {
    proxy_client: reqwest::Client,
    non_proxy_client: reqwest::Client,
    whitelisted_urls: Vec<String>,
    request_id: Option<String>,
}

impl ProxyClient {
        /// Constructs a new ApiClient with the provided proxy configuration and whitelisted URLs.
    pub fn new(
        proxy_config: Proxy,
        whitelisted_urls: Vec<String>,
    ) -> CustomResult<Self, ApiClientError> {
        let non_proxy_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .into_report()
            .change_context(ApiClientError::ClientConstructionFailed)?;

        let mut proxy_builder =
            reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());

        if let Some(url) = proxy_config.https_url.as_ref() {
            proxy_builder = proxy_builder.proxy(
                reqwest::Proxy::https(url)
                    .into_report()
                    .change_context(ApiClientError::InvalidProxyConfiguration)?,
            );
        }

        if let Some(url) = proxy_config.http_url.as_ref() {
            proxy_builder = proxy_builder.proxy(
                reqwest::Proxy::http(url)
                    .into_report()
                    .change_context(ApiClientError::InvalidProxyConfiguration)?,
            );
        }

        let proxy_client = proxy_builder
            .build()
            .into_report()
            .change_context(ApiClientError::InvalidProxyConfiguration)?;
        Ok(Self {
            proxy_client,
            non_proxy_client,
            whitelisted_urls,
            request_id: None,
        })
    }

        /// Returns a reqwest client based on the provided base URL and optional client certificate and key.
    /// If both client certificate and key are provided, it constructs a reqwest client with the provided certificate and key,
    /// otherwise, it checks if the base URL is whitelisted and returns a non-proxy client or a proxy client accordingly.
    pub fn get_reqwest_client(
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
            (_, _) => {
                if self.whitelisted_urls.contains(&base_url) {
                    Ok(self.non_proxy_client.clone())
                } else {
                    Ok(self.proxy_client.clone())
                }
            }
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
        /// Takes a mutable reference to self and a serde_json::Value object `body`. 
    /// Updates the inner field of self by mapping over it, calling the json method of the inner 
    /// with the `body` parameter. 
    fn json(&mut self, body: serde_json::Value) {
        self.inner = self.inner.take().map(|r| r.json(&body));
    }
        /// Converts the given `serde_json::Value` into a url-encoded form and sets it as the body of the request.
    fn url_encoded_form(&mut self, body: serde_json::Value) {
        self.inner = self.inner.take().map(|r| r.form(&body));
    }

        /// Sets a timeout for the inner value, if it exists.
    ///
    /// If the inner value exists, this method sets a timeout for it using the specified Duration. If the inner value is None, the method does nothing.
    ///
    /// # Arguments
    /// * `timeout` - The Duration for the timeout
    ///
    fn timeout(&mut self, timeout: Duration) {
        self.inner = self.inner.take().map(|r| r.timeout(timeout));
    }

        /// Parses the request body as a multipart form and modifies the inner state with the parsed form data.
    /// 
    /// # Arguments
    /// 
    /// * `form` - The form data to be parsed from the request body.
    /// 
    fn multipart(&mut self, form: Form) {
        self.inner = self.inner.take().map(|r| r.multipart(form));
    }

        /// Adds a custom header to the request. If the value is sensitive, it will be masked before adding to the header.
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

        /// Sends the HTTP request and returns a future containing the response or an error.
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
#[async_trait::async_trait]
impl ApiClient for ProxyClient {
        /// Makes a request to the specified URL using the given HTTP method. Returns a custom result containing a boxed trait object implementing RequestBuilder, or an ApiClientError if an error occurs.
    fn request(
        &self,
        method: Method,
        url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        self.request_with_certificate(method, url, None, None)
    }

        /// Makes a request using the provided HTTP method, URL, and optional client certificate and key.
    /// If a client certificate and key are provided, they are used to construct the request client. 
    /// Returns a Result containing a Boxed RequestBuilder or an ApiClientError if the client construction fails.
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

    /// Asynchronously sends a request using the provided state and request parameters. It also allows for setting a timeout in seconds and specifies whether to forward the request to Kafka.
    ///
    /// # Arguments
    ///
    /// * `state` - The application state used for sending the request.
    /// * `request` - The request to be sent.
    /// * `option_timeout_secs` - An optional timeout in seconds for the request.
    /// * `_forward_to_kafka` - A boolean indicating whether to forward the request to Kafka.
    ///
    /// # Returns
    ///
    /// A Result containing a reqwest::Response if the request is successful, or an ApiClientError if an error occurs.
    ///
    async fn send_request(
        &self,
        state: &AppState,
        request: Request,
        option_timeout_secs: Option<u64>,
        _forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError> {
        crate::services::send_request(state, request, option_timeout_secs).await
    }

        /// Adds a request ID to the current instance.
    ///
    /// # Arguments
    ///
    /// * `request_id` - The request ID to be added to the current instance.
    ///
    fn add_request_id(&mut self, request_id: RequestId) {
        self.request_id
            .replace(request_id.as_hyphenated().to_string());
    }

        /// Returns the request ID associated with the current instance, if available.
    /// 
    /// # Returns
    /// 
    /// - `Some(String)`: If a request ID is available, returns it as a `String`.
    /// - `None`: If no request ID is available, returns `None`.
    fn get_request_id(&self) -> Option<String> {
        self.request_id.clone()
    }

        /// Adds a merchant ID to the struct.
    /// 
    /// # Arguments
    /// * `_merchant_id` - An optional string containing the merchant ID to be added.
    fn add_merchant_id(&mut self, _merchant_id: Option<String>) {}

    /// Adds a flow name to the current object.
    ///
    /// # Arguments
    ///
    /// * `_flow_name` - A String representing the name of the flow to be added.
    ///
    fn add_flow_name(&mut self, _flow_name: String) {}
}

///
/// Api client for testing sending request
///
#[derive(Clone)]
pub struct MockApiClient;

#[async_trait::async_trait]
impl ApiClient for MockApiClient {
        /// Makes a request to the specified URL using the given HTTP method.
    /// 
    /// # Arguments
    /// * `method` - The HTTP method to use for the request.
    /// * `url` - The URL to make the request to.
    /// 
    /// # Returns
    /// A `Result` containing a `Box` of a trait object implementing `RequestBuilder` on success, or an `ApiClientError` on failure.
    fn request(
        &self,
        _method: Method,
        _url: String,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        // [#2066]: Add Mock implementation for ApiClient
        Err(ApiClientError::UnexpectedState.into())
    }

        /// This method makes a request using a certificate for authentication, if provided. It takes the HTTP method, URL, certificate, and certificate key as parameters and returns a Result containing a RequestBuilder or an ApiClientError in case of an unexpected state.
    fn request_with_certificate(
        &self,
        _method: Method,
        _url: String,
        _certificate: Option<String>,
        _certificate_key: Option<String>,
    ) -> CustomResult<Box<dyn RequestBuilder>, ApiClientError> {
        // [#2066]: Add Mock implementation for ApiClient
        Err(ApiClientError::UnexpectedState.into())
    }

        /// Sends a request to the API server, with the option to set a timeout and forward the request to Kafka.
    /// 
    /// # Arguments
    /// * `state` - The current application state
    /// * `request` - The request to be sent
    /// * `option_timeout_secs` - An optional timeout in seconds
    /// * `forward_to_kafka` - A boolean indicating whether to forward the request to Kafka
    /// 
    /// # Returns
    /// A Result containing the reqwest::Response if the request was successful, or an ApiClientError if an unexpected state occurred.
    /// 
    async fn send_request(
        &self,
        _state: &AppState,
        _request: Request,
        _option_timeout_secs: Option<u64>,
        _forward_to_kafka: bool,
    ) -> CustomResult<reqwest::Response, ApiClientError> {
        // [#2066]: Add Mock implementation for ApiClient
        Err(ApiClientError::UnexpectedState.into())
    }

    /// Adds a request ID to the ApiClient.
    ///
    /// This method takes a request ID and adds it to the ApiClient.
    ///
    /// # Arguments
    ///
    /// * `_request_id` - The request ID to be added to the ApiClient
    ///
    fn add_request_id(&mut self, _request_id: RequestId) {
        // [#2066]: Add Mock implementation for ApiClient
    }

        /// Retrieves the request ID associated with the ApiClient.
    /// Returns an Option containing the request ID if one exists, otherwise returns None.
    fn get_request_id(&self) -> Option<String> {
        // [#2066]: Add Mock implementation for ApiClient
        None
    }

    fn add_merchant_id(&mut self, _merchant_id: Option<String>) {}

    fn add_flow_name(&mut self, _flow_name: String) {}
}
