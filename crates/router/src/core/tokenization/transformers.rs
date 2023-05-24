use error_stack::ResultExt;
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers, services,
    types::{api::tokenization, storage},
    utils::{self, OptionExt},
};

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTridRequest {
    pub locker_id: String,
    pub locker_name: String,
    pub network: String,
    pub parent_token_requestor_id: Option<String>,
    pub test_mode: bool,
    pub company_details: tokenization::CompanyDetails,
    pub merchant_id: String,
    pub se_number: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTridResponse {
    pub status: String,
    pub response_status: String,
    pub response_message: Option<String>,
    pub response_code: Option<String>,
    pub network_name: String,
    pub onboarding_date: Option<String>,
    pub last_modified: Option<String>,
}

pub fn get_trid_request(
    locker: &settings::Locker,
    merchant_account: &storage::MerchantAccount,
    req: &tokenization::GetTrid,
) -> CustomResult<services::Request, errors::TokenizationError> {
    let locker_id = merchant_account
        .locker_id
        .clone()
        .get_required_value("locker_id")
        .change_context(errors::TokenizationError::GetTridFailed)?;
    let locker_name = merchant_account
        .locker_name
        .clone()
        .get_required_value("locker_name")
        .change_context(errors::TokenizationError::GetTridFailed)?;
    let payload = GetTridRequest {
        locker_id,
        locker_name,
        network: req.network.to_owned(),
        parent_token_requestor_id: None,
        test_mode: true, //FIXME
        company_details: req.company_details.to_owned(),
        merchant_id: merchant_account.merchant_id.to_owned(),
        se_number: req.se_number.to_owned(),
    };
    let body = utils::Encode::<GetTridRequest>::encode_to_string_of_json(&payload)
        .change_context(errors::TokenizationError::GetTridFailed)?;
    let mut url = locker.tokenization_host.to_owned();
    url.push_str("/tokenization/getTrid");
    let mut request = services::Request::new(services::Method::Post, &url);
    request.add_header(headers::CONTENT_TYPE, "application/x-www-form-urlencoded");
    request.set_body(body);
    Ok(request)
}
