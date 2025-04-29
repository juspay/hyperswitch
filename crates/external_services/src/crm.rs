use std::sync::Arc;

use common_utils::{
    errors::CustomResult,
    ext_traits::ConfigExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use http::header;
use hyperswitch_interfaces::{
    crm::{CRMInterface, CRMPayload},
    errors::HttpClientError,
    types::Proxy,
};
use masking::PeekInterface;
use reqwest;
use router_env::logger;

use crate::{
    http_client, http_client::request::InvalidCRMConfig, hubspot_proxy::core::HubspotRequest,
};

/// Hubspot CRM configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HubspotSettings {
    /// The ID of the Hubspot form to be submitted.
    pub form_id: String,

    /// The URL to which the Hubspot form data will be sent.
    pub request_url: String,
}

impl HubspotSettings {
    /// Validates Hubspot configuration
    pub(super) fn validate(&self) -> Result<(), InvalidCRMConfig> {
        use common_utils::fp_utils::when;

        when(self.request_url.is_default_or_empty(), || {
            Err(InvalidCRMConfig("request url must not be empty"))
        })?;

        when(self.form_id.is_default_or_empty(), || {
            Err(InvalidCRMConfig("form_id must not be empty"))
        })
    }
}

#[derive(Debug, Clone)]
/// NoCrm struct
pub struct NoCrm;

/// Enum representing different CRM configurations
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "crm_manager")]
#[serde(rename_all = "snake_case")]
pub enum CRMManagerConfig {
    /// Hubspot CRM configuration
    HubspotProxy {
        /// Hubspot CRM configuration
        hubspot_proxy: HubspotSettings,
    },

    /// No CRM configuration
    #[default]
    NoCrm,
}

impl CRMManagerConfig {
    /// Verifies that the client configuration is usable
    pub fn validate(&self) -> Result<(), InvalidCRMConfig> {
        match self {
            Self::HubspotProxy { hubspot_proxy } => hubspot_proxy.validate(),
            Self::NoCrm => Ok(()),
        }
    }

    /// Retrieves the appropriate CRM client based on the configuration.
    pub async fn get_crm_client(&self) -> Arc<dyn CRMInterface> {
        match self {
            Self::HubspotProxy { hubspot_proxy } => Arc::new(hubspot_proxy.clone()),
            Self::NoCrm => Arc::new(NoCrm),
        }
    }
}

#[async_trait::async_trait]
impl CRMInterface for NoCrm {
    async fn make_body(&self, _details: CRMPayload) -> RequestContent {
        RequestContent::Json(Box::new(serde_json::json!({})))
    }

    async fn make_request(&self, _body: RequestContent, _origin_base_url: String) -> Request {
        RequestBuilder::default().build()
    }

    async fn send_request(
        &self,
        _proxy: &Proxy,
        _request: Request,
    ) -> CustomResult<reqwest::Response, HttpClientError> {
        logger::info!("NO CRM manager is not configured, resolving with a 200 OK response");
        Err(HttpClientError::UnexpectedState)
            .attach_printable("NO CRM manager is not configured, resolving with a 200 OK response")
    }
}

#[async_trait::async_trait]
impl CRMInterface for HubspotSettings {
    async fn make_body(&self, details: CRMPayload) -> RequestContent {
        RequestContent::FormUrlEncoded(Box::new(HubspotRequest::new(
            details.business_country_name.unwrap_or_default(),
            self.form_id.clone(),
            details.poc_name.unwrap_or_default(),
            details
                .poc_email
                .clone()
                .unwrap_or_default()
                .peek()
                .to_string(),
            details.legal_business_name.unwrap_or_default(),
            details.business_website.unwrap_or_default(),
        )))
    }

    async fn make_request(&self, body: RequestContent, origin_base_url: String) -> Request {
        RequestBuilder::new()
            .method(Method::Post)
            .url(self.request_url.as_str())
            .set_body(body)
            .attach_default_headers()
            .headers(vec![(
                header::ORIGIN.to_string(),
                format!("{origin_base_url}/dashboard").into(),
            )])
            .build()
    }

    async fn send_request(
        &self,
        proxy: &Proxy,
        request: Request,
    ) -> CustomResult<reqwest::Response, HttpClientError> {
        http_client::send_request(proxy, request, None).await
    }
}
