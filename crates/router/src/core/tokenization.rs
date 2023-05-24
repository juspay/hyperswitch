mod transformers;
use common_utils::ext_traits::BytesExt;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    consts,
    core::errors::{self, RouterResponse, RouterResult, StorageErrorExt},
    routes::AppState,
    services,
    types::{api::tokenization, storage},
    utils,
};

#[instrument(skip_all)]
pub async fn get_trid_core(
    state: &AppState,
    merchant_account: storage::merchant_account::MerchantAccount,
    req: tokenization::GetTrid,
) -> RouterResponse<tokenization::GetTridResponse> {
    let db = &*state.store;
    let merchant_id = &merchant_account.merchant_id;
    let token_locker_id = utils::generate_id(consts::ID_LENGTH, "tlid");
    let locker_name = format!(
        "{merchant_id}_{}",
        utils::generate_id(consts::ID_LENGTH, "")
    );
    let updated_merchant_account = storage::MerchantAccountUpdate::TokenizationUpdate {
        token_locker_id: Some(token_locker_id),
        locker_name: Some(locker_name),
    };
    let merchant_account = db
        .update_specific_fields_in_merchant(merchant_id, updated_merchant_account)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    let response = call_tokenization_trid(state, &merchant_account, req).await?;
    Ok(services::ApplicationResponse::Json(response))
}

pub async fn call_tokenization_trid(
    state: &AppState,
    merchant_account: &storage::MerchantAccount,
    req: tokenization::GetTrid,
) -> RouterResult<tokenization::GetTridResponse> {
    let locker = &state.conf.locker;
    let get_trid_request = transformers::get_trid_request(locker, merchant_account, &req)
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let response = services::call_connector_api(state, get_trid_request)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;
    let response: transformers::GetTridResponse = match response {
        Ok(trid) => trid
            .response
            .parse_struct("GetTridResponse")
            .change_context(errors::ApiErrorResponse::InternalServerError),
        Err(_) => Err(report!(errors::ApiErrorResponse::InternalServerError)),
    }?;
    let get_trid_response = tokenization::GetTridResponse {
        status: response.status,
        network: response.network_name,
        company_details: req.company_details,
        se_number: req.se_number,
        response_status: response.response_status,
        response_message: response.response_message,
        response_code: response.response_code,
        onboarding_date: response.onboarding_date,
        last_modified: response.last_modified,
    };
    Ok(get_trid_response)
}
