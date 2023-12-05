pub mod utils;
use api_models::pm_blacklist;
use common_utils::{errors::CustomResult, ext_traits::Encode};
use error_stack::ResultExt;

use crate::{
    core::errors::{self, api_error_response},
    headers, logger,
    routes::AppState,
    services, types::{self, domain},
};

pub async fn block_payment_method(
    state: AppState,
    _req: &actix_web::HttpRequest,
    body: pm_blacklist::BlacklistPmRequest,
    merchant_account: domain::MerchantAccount
) -> CustomResult<
    services::ApplicationResponse<pm_blacklist::BlacklistPmResponse>,
    api_error_response::ApiErrorResponse,
> {
    let fingerprints_to_block = body.pm_to_block;
    println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>> {:?}",merchant_account);

    Ok(services::api::ApplicationResponse::Json(
        pm_blacklist::BlacklistPmResponse { status_message: "success".to_string() , fingerprints_blocked: fingerprints_to_block} 
    ))

}

