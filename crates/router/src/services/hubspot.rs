use common_utils::request::{Method, RequestBuilder, RequestContent};
use error_stack::ResultExt;
use http::header;

use crate::{
    consts::user as user_consts,
    core::errors::UserErrors,
    routes::SessionState,
    services::send_request,
    utils::user::{self as user_utils},
};

#[derive(Clone, Debug, serde::Serialize, Default)]
pub struct HubspotRequest {
    #[serde(rename = "useHubspot")]
    pub use_hubspot: bool,
    pub country: String,
    #[serde(rename = "hubspotFormId")]
    pub hubspot_form_id: String,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    #[serde(rename = "companyName")]
    pub company_name: String,
    pub lead_source: String,
    pub website: String,
    pub phone: String,
    pub role: String,
    #[serde(rename = "monthlyGMV")]
    pub monthly_gmv: String,
    pub bd_notes: String,
    pub message: String,
}

impl HubspotRequest {
    pub fn new(
        country: String,
        hubspot_form_id: String,
        firstname: String,
        email: String,
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
            lead_source: user_consts::HUBSPOT_LEAD_SOURCE.to_string(),
            website,
            ..Default::default()
        }
    }

    pub async fn create_and_send_request(self, state: &SessionState, user_id: String) {
        let base_url = user_utils::get_base_url(state);

        let hubspot_request = RequestBuilder::new()
            .method(Method::Post)
            .url(&state.conf.hubspot.request_url)
            .set_body(RequestContent::FormUrlEncoded(Box::new(self)))
            .attach_default_headers()
            .headers(vec![(
                header::ORIGIN.to_string(),
                format!("{base_url}/dashboard").into(),
            )])
            .build();

        let _ = send_request(state, hubspot_request, None)
            .await
            .change_context(UserErrors::InternalServerError)
            .attach_printable(format!(
                "Failed to send data to hubspot for user_id {}",
                user_id,
            ));
    }
}
