use masking::Secret;

/// Lead source constant for Hubspot
pub const HUBSPOT_LEAD_SOURCE: &str = "Hyperswitch Dashboard";

/// Struct representing a request to Hubspot
#[derive(Clone, Debug, serde::Serialize, Default)]
pub struct HubspotRequest {
    /// Indicates whether Hubspot should be used.
    #[serde(rename = "useHubspot")]
    pub use_hubspot: bool,

    /// The country of the user or company.
    pub country: String,

    /// The ID of the Hubspot form being submitted.
    #[serde(rename = "hubspotFormId")]
    pub hubspot_form_id: String,

    /// The first name of the user.
    pub firstname: Secret<String>,

    /// The last name of the user.
    pub lastname: Secret<String>,

    /// The email address of the user.
    pub email: Secret<String>,

    /// The name of the company.
    #[serde(rename = "companyName")]
    pub company_name: String,

    /// The source of the lead, typically set to "Hyperswitch Dashboard".
    pub lead_source: String,

    /// The website URL of the company.
    pub website: String,

    /// The phone number of the user.
    pub phone: Secret<String>,

    /// The role or designation of the user.
    pub role: String,

    /// The monthly GMV (Gross Merchandise Value) of the company.
    #[serde(rename = "monthlyGMV")]
    pub monthly_gmv: String,

    /// Notes from the business development team.
    pub bd_notes: String,

    /// Additional message or comments.
    pub message: String,
}

#[allow(missing_docs)]
impl HubspotRequest {
    pub fn new(
        country: String,
        hubspot_form_id: String,
        firstname: Secret<String>,
        email: Secret<String>,
        company_name: String,
        website: String,
    ) -> Self {
        Self {
            use_hubspot: true,
            country,
            hubspot_form_id,
            firstname,
            email,
            company_name,
            lead_source: HUBSPOT_LEAD_SOURCE.to_string(),
            website,
            ..Default::default()
        }
    }
}
