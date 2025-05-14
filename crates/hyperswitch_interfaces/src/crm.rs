use common_enums::CountryAlpha2;
use common_utils::{
    errors::CustomResult,
    request::{Request, RequestContent},
};
use masking::Secret;

use super::types::Proxy;
use crate::errors::HttpClientError;

/// Crm Payload structure
#[derive(Clone, Debug, serde::Serialize, Default)]
pub struct CrmPayload {
    /// The legal name of the business.
    pub legal_business_name: Option<String>,

    /// A label or tag associated with the business.
    pub business_label: Option<String>,

    /// The location of the business, represented as a country code (ISO Alpha-2 format).
    pub business_location: Option<CountryAlpha2>,

    /// The display name of the business.
    pub display_name: Option<String>,

    /// The email address of the point of contact (POC) for the business.
    pub poc_email: Option<Secret<String>>,

    /// The type of business (e.g., LLC, Corporation, etc.).
    pub business_type: Option<String>,

    /// A unique identifier for the business.
    pub business_identifier: Option<String>,

    /// The website URL of the business.
    pub business_website: Option<String>,

    /// The name of the point of contact (POC) for the business.
    pub poc_name: Option<Secret<String>>,

    /// The contact number of the point of contact (POC) for the business.
    pub poc_contact: Option<Secret<String>>,

    /// Additional comments or notes about the business.
    pub comments: Option<String>,

    /// Indicates whether the Crm process for the business is completed.
    pub is_completed: bool,

    /// The name of the country where the business is located.
    pub business_country_name: Option<String>,
}

/// Trait defining the interface for encryption management
#[async_trait::async_trait]
pub trait CrmInterface: Send + Sync {
    /// Make body for the request
    async fn make_body(&self, details: CrmPayload) -> RequestContent;

    /// Encrypt the given input data
    async fn make_request(&self, body: RequestContent, origin_base_url: String) -> Request;

    /// Decrypt the given input data
    async fn send_request(
        &self,
        proxy: &Proxy,
        request: Request,
    ) -> CustomResult<reqwest::Response, HttpClientError>;
}
