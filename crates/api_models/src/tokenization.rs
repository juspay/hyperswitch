use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Default, Debug, ToSchema, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetTrid {
    /// CardBrand
    pub network: String,
    /// The company/merchant details for onboarding
    pub company_details: CompanyDetails,
    /// Amex is onboarded manually This number will be received from AMEX upon sharing company details
    pub se_number: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct CompanyDetails {
    /// Legal Name of the company
    pub legal_name: String,
    /// Trade Name of the company
    pub trade_name: String,
    /// Url of the company
    pub website_url: String,
    /// City of the company
    pub city: String,
    /// Country code
    pub country_code: String,
    /// Contact email of the company
    pub contact_email: String,
    /// Business Identification type(PAN)
    pub business_identification_type: String,
    /// PAN number
    pub business_identification_value: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetTridResponse {
    pub status: String,
    pub network: String,
    pub company_details: CompanyDetails,
    pub se_number: Option<String>,
    pub response_status: String,
    pub response_message: Option<String>,
    pub response_code: Option<String>,
    pub onboarding_date: Option<String>,
    pub last_modified: Option<String>,
}
