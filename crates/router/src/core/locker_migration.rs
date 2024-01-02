use api_models::{enums as api_enums, locker_migration::MigrateCardResponse};
use common_utils::errors::CustomResult;
use diesel_models::{enums as storage_enums, PaymentMethod};
use error_stack::{FutureExt, ResultExt};
use futures::TryFutureExt;

use super::{errors::StorageErrorExt, payment_methods::cards};
use crate::{
    errors,
    routes::AppState,
    services::{self, logger},
    types::{api, domain},
};

pub async fn rust_locker_migration(
    state: AppState,
    merchant_id: &str,
) -> CustomResult<services::ApplicationResponse<MigrateCardResponse>, errors::ApiErrorResponse> {
    let db = state.store.as_ref();

    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let domain_customers = db
        .list_customers_by_merchant_id(merchant_id, &key_store)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut customers_moved = 0;
    let mut cards_moved = 0;

    for customer in domain_customers {
        let result = db
            .find_payment_method_by_customer_id_merchant_id_list(&customer.customer_id, merchant_id)
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .and_then(|pm| {
                call_to_locker(
                    &state,
                    pm,
                    &customer.customer_id,
                    merchant_id,
                    &merchant_account,
                )
            })
            .await?;

        customers_moved += 1;
        cards_moved += result;
    }

    Ok(services::api::ApplicationResponse::Json(
        MigrateCardResponse {
            status_code: "200".to_string(),
            status_message: "Card migration completed".to_string(),
            customers_moved,
            cards_moved,
        },
    ))
}

pub async fn call_to_locker(
    state: &AppState,
    payment_methods: Vec<PaymentMethod>,
    customer_id: &String,
    merchant_id: &str,
    merchant_account: &domain::MerchantAccount,
) -> CustomResult<usize, errors::ApiErrorResponse> {
    let mut cards_moved = 0;

    for pm in payment_methods
        .into_iter()
        .filter(|pm| matches!(pm.payment_method, storage_enums::PaymentMethod::Card))
    {
        let card =
            cards::get_card_from_locker(state, customer_id, merchant_id, &pm.payment_method_id)
                .await;

        let card = match card {
            Ok(card) => card,
            Err(err) => {
                logger::error!("Failed to fetch card from Basilisk HS locker : {:?}", err);
                continue;
            }
        };

        let card_details = api::CardDetail {
            card_number: card.card_number,
            card_exp_month: card.card_exp_month,
            card_exp_year: card.card_exp_year,
            card_holder_name: card.name_on_card,
            nick_name: card.nick_name.map(masking::Secret::new),
            card_issuing_country: None,
            card_network: None,
            card_issuer: None,
            card_type: None,
        };

        let pm_create = api::PaymentMethodCreate {
            payment_method: pm.payment_method,
            payment_method_type: pm.payment_method_type,
            payment_method_issuer: pm.payment_method_issuer,
            payment_method_issuer_code: pm.payment_method_issuer_code,
            card: Some(card_details.clone()),
            wallet: None,
            bank_transfer: None,
            metadata: pm.metadata,
            customer_id: pm.customer_id,
            card_network: card.card_brand,
        };

        let add_card_result = cards::add_card_hs(
                state,
                pm_create,
                &card_details,
                customer_id.to_string(),
                merchant_account,
                api_enums::LockerChoice::Tartarus,
                Some(&pm.payment_method_id),
            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "Card migration failed for merchant_id: {merchant_id}, customer_id: {customer_id}, payment_method_id: {} ",
                pm.payment_method_id
            ));

        let (_add_card_rs_resp, _is_duplicate) = match add_card_result {
            Ok(output) => output,
            Err(err) => {
                logger::error!("Failed to add card to Rust locker : {:?}", err);
                continue;
            }
        };

        cards_moved += 1;

        logger::info!(
                "Card migrated for merchant_id: {merchant_id}, customer_id: {customer_id}, payment_method_id: {} ",
                pm.payment_method_id
            );
    }

    Ok(cards_moved)
}
