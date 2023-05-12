use error_stack::ResultExt;

use crate::{
    core::{
        errors::{self, RouterResult},
        payment_methods::{cards, vault},
    },
    routes::AppState,
    types::{api, domain, storage},
    utils,
};

pub async fn make_payout_data<'a>(
    state: &'a AppState,
    request: &api::PayoutCreateRequest,
    payout_create: &storage::PayoutCreate,
) -> RouterResult<Option<api::PayoutMethodData>> {
    let db = &*state.store;
    match (
        request.payout_method_data.to_owned(),
        payout_create.payout_token.to_owned(),
    ) {
        (None, Some(payout_token)) => {
            let (pm, supplementary_data) = vault::Vault::get_payout_method_data_from_locker(
                state,
                &payout_token,
            )
            .await
            .attach_printable(
                "Payout method for given token not found or there was a problem fetching it",
            )?;
            utils::when(
                supplementary_data
                    .customer_id
                    .ne(&Some(payout_create.customer_id.to_owned())),
                || {
                    Err(errors::ApiErrorResponse::PreconditionFailed { message: "customer associated with payout method and customer passed in payout are not same".into() })
                },
            )?;
            Ok(pm)
        }
        (Some(payout_method), None) => {
            let payout_token = vault::Vault::store_payout_method_data_in_locker(
                state,
                None,
                &payout_method,
                Some(payout_create.customer_id.to_owned()),
                payout_create.payout_type,
            )
            .await?;
            //FIXME: we should have Status field in payout_create and update status from require_payout_method_data to require_fulfillment
            let payout_update = storage::PayoutCreateUpdate::PayoutTokenUpdate { payout_token };
            db.update_payout_create_by_merchant_id_payout_id(
                &payout_create.merchant_id,
                &payout_create.payout_id,
                payout_update,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable("Error updating token in payout create")?;
            Ok(Some(payout_method))
        }
        _ => Ok(None),
    }
}

pub async fn save_payout_data_to_locker(
    state: &AppState,
    payout_create: &storage::payout_create::PayoutCreate,
    payout_method_data: &api::PayoutMethodData,
    merchant_account: &domain::MerchantAccount,
) -> RouterResult<Option<String>> {
    match payout_method_data {
        api_models::payouts::PayoutMethodData::Card(card) => {
            let card_details = api::CardDetail {
                card_number: card.card_number.to_owned(),
                card_exp_month: card.expiry_month.to_owned(),
                card_exp_year: card.expiry_year.to_owned(),
                card_holder_name: Some(card.card_holder_name.to_owned()),
            };
            let stored_card_resp = cards::call_to_card_hs(
                state,
                &card_details,
                &payout_create.customer_id,
                merchant_account,
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)?;
            match stored_card_resp.duplicate {
                Some(true) => Ok(Some(stored_card_resp.card_reference)),
                _ => Ok(None),
            }
        }
        api_models::payouts::PayoutMethodData::Bank(_) => Ok(None), //To be implemented after bank storage support in basilisk-hs
    }
}
