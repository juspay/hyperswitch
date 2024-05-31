use common_utils::{
    errors::CustomResult,
    request::{Request, RequestContent},
};
use error_stack::ResultExt;
use masking::Secret;
use storage_impl::errors;

use super::send_request;
use crate::routes::{app::AppStateInfo, AppState};

mod types;

const TAG: &str = "hyperswitch";

/*
 *
 *

pub struct Request {
    pub url: String,
    pub headers: Headers,
    pub method: Method,
    pub certificate: Option<Secret<String>>,
    pub certificate_key: Option<Secret<String>>,
    pub body: Option<RequestContent>,
 }

 *
 *
 */

pub async fn register_api_key(
    state: &AppState,
    api_key: Secret<String>,
    merchant_id: String,
    key_id: String,
) -> CustomResult<(), errors::ApiClientError> {
    let request = types::RuleRequest::ApiKey {
        tag: TAG.to_string(),
        api_key,
        identifiers: types::ApiKeyIdentifier::ApiKey {
            merchant_id,
            key_id,
        },
    };

    let mut request_builder = Request::new(
        common_utils::request::Method::Post,
        &state.conf().api_keys.get_inner().decision_url,
    );

    request_builder.set_body(RequestContent::Json(Box::new(request)));

    let response = send_request(state, request_builder, Some(10)).await?;

    let _output = response
        .json::<types::RuleResponse>()
        .await
        .change_context(errors::ApiClientError::ResponseDecodingFailed)?;

    Ok(())
}
