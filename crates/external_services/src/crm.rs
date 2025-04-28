use std::sync::Arc;

use crate::http_client;
use crate::http_client::request::InvalidCRMConfig;
use crate::hubspot_proxy::core::HubspotRequest;
use common_utils::request::Request;
use common_utils::request::{Method, RequestBuilder, RequestContent};
use common_utils::{errors::CustomResult, ext_traits::ConfigExt};
use http::header;
use hyperswitch_interfaces::crm::{CRMInterface, CRMPayload};
use hyperswitch_interfaces::errors::HttpClientError;
use hyperswitch_interfaces::types::Proxy;
use masking::PeekInterface;
use reqwest;

/// Hubspot CRM configuration
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HubspotSettings {
    /// The ID of the Hubspot form to be submitted.
    pub form_id: String,

    /// The URL to which the Hubspot form data will be sent.
    pub request_url: String,
}

impl HubspotSettings {
    /// Validates the AWS S3 file storage configuration.
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
/// NoCRM struct
pub struct NoCRM;

/// Enum representing different CRM configurations
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub enum CRMManagerConfig {
    /// Hubspot CRM configuration
    HubspotProxy {
        /// Hubspot CRM configuration
        hubspot_proxy: HubspotSettings,
    },

    /// No CRM configuration
    #[default]
    NoCRM,
}

impl CRMManagerConfig {
    /// Verifies that the client configuration is usable
    pub fn validate(&self) -> Result<(), InvalidCRMConfig> {
        match self {
            Self::HubspotProxy { hubspot_proxy } => hubspot_proxy.validate(),
            Self::NoCRM => Ok(()),
        }
    }

    /// Retrieves the appropriate encryption client based on the configuration.
    pub async fn get_crm_client(&self) -> Arc<dyn CRMInterface> {
        match self {
            Self::HubspotProxy { hubspot_proxy } => Arc::new(hubspot_proxy.clone()),
            Self::NoCRM => Arc::new(NoCRM),
        }
    }
}

#[async_trait::async_trait]
impl CRMInterface for NoCRM {
    async fn make_body(&self, _details: CRMPayload) -> RequestContent {
        todo!()
    }

    async fn make_request(&self, _body: RequestContent, _origin_base_url: String) -> Request {
        todo!()
    }

    async fn send_request(
        &self,
        _proxy: &Proxy,
        _request: Request,
    ) -> CustomResult<reqwest::Response, HttpClientError> {
        todo!()
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
