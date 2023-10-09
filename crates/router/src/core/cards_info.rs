use common_utils::fp_utils::when;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse},
        payments::helpers,
    },
    routes,
    services::ApplicationResponse,
    types::{domain, transformers::ForeignFrom},
};

fn verify_iin_length(card_iin: &str) -> Result<(), errors::ApiErrorResponse> {
    let is_bin_length_in_range = card_iin.len() == 6 || card_iin.len() == 8;
    when(!is_bin_length_in_range, || {
        Err(errors::ApiErrorResponse::InvalidCardIinLength)
    })
}

#[instrument(skip_all)]
pub async fn retrieve_card_info(
    state: routes::AppState,
    merchant_account: domain::MerchantAccount,
    request: api_models::cards_info::CardsInfoRequest,
) -> RouterResponse<api_models::cards_info::CardInfoResponse> {
    let db = state.store.as_ref();

    verify_iin_length(&request.card_iin)?;
    helpers::verify_payment_intent_time_and_client_secret(
        db,
        &merchant_account,
        request.client_secret,
    )
    .await?;

    let card_info = db
        .get_card_info(&request.card_iin)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to retrieve card information")?
        .ok_or(report!(errors::ApiErrorResponse::InvalidCardIin))?;

    Ok(ApplicationResponse::Json(
        api_models::cards_info::CardInfoResponse::foreign_from(card_info),
    ))
}
