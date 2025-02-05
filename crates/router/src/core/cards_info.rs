use api_models::cards_info as cards_info_api_types;
use common_utils::fp_utils::when;
use diesel_models::cards_info as storage;
use error_stack::{report, ResultExt};
use router_env::{instrument, tracing};

use crate::{
    core::{
        errors::{self, RouterResponse, StorageErrorExt},
        payments::helpers,
    },
    db::cards_info::CardsInfoInterface,
    routes,
    services::ApplicationResponse,
    types::{
        domain,
        transformers::{ForeignFrom, ForeignInto},
    },
};

fn verify_iin_length(card_iin: &str) -> Result<(), errors::ApiErrorResponse> {
    let is_bin_length_in_range = card_iin.len() == 6 || card_iin.len() == 8;
    when(!is_bin_length_in_range, || {
        Err(errors::ApiErrorResponse::InvalidCardIinLength)
    })
}

#[instrument(skip_all)]
pub async fn retrieve_card_info(
    state: routes::SessionState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    request: cards_info_api_types::CardsInfoRequest,
) -> RouterResponse<cards_info_api_types::CardInfoResponse> {
    let db = state.store.as_ref();

    verify_iin_length(&request.card_iin)?;
    helpers::verify_payment_intent_time_and_client_secret(
        &state,
        &merchant_account,
        &key_store,
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
        cards_info_api_types::CardInfoResponse::foreign_from(card_info),
    ))
}

#[instrument(skip_all)]
pub async fn create_card_info(
    state: routes::SessionState,
    card_info_request: cards_info_api_types::CardInfoCreateRequest,
) -> RouterResponse<cards_info_api_types::CardInfoResponse> {
    let db = state.store.as_ref();
    CardsInfoInterface::add_card_info(db, card_info_request.foreign_into())
        .await
        .to_duplicate_response(errors::ApiErrorResponse::GenericDuplicateError {
            message: "CardInfo with given key already exists in our records".to_string(),
        })
        .map(|card_info| ApplicationResponse::Json(card_info.foreign_into()))
}

#[instrument(skip_all)]
pub async fn update_card_info(
    state: routes::SessionState,
    card_info_request: cards_info_api_types::CardInfoUpdateRequest,
) -> RouterResponse<cards_info_api_types::CardInfoResponse> {
    let db = state.store.as_ref();
    let cards_info_api_types::CardInfoUpdateRequest {
        card_iin,
        card_issuer,
        card_network,
        card_type,
        card_subtype,
        card_issuing_country,
        bank_code_id,
        bank_code,
        country_code,
        last_updated_provider,
    } = card_info_request;
    CardsInfoInterface::update_card_info(
        db,
        card_iin,
        storage::UpdateCardInfo {
            card_issuer,
            card_network,
            card_type,
            card_subtype,
            card_issuing_country,
            bank_code_id,
            bank_code,
            country_code,
            last_updated: Some(common_utils::date_time::now()),
            last_updated_provider,
        },
    )
    .await
    .to_not_found_response(errors::ApiErrorResponse::GenericNotFoundError {
        message: "GSM with given key does not exist in our records".to_string(),
    })
    .attach_printable("Failed while updating Gsm rule")
    .map(|card_info| ApplicationResponse::Json(card_info.foreign_into()))
}
