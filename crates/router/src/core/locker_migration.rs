#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use api_models::enums as api_enums;
use api_models::locker_migration::MigrateCardResponse;
use common_utils::{errors::CustomResult, id_type};
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use diesel_models::enums as storage_enums;
use diesel_models::PaymentMethod;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use error_stack::FutureExt;
use error_stack::ResultExt;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use futures::TryFutureExt;

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use super::errors::StorageErrorExt;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use super::payment_methods::cards;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use crate::services::logger;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
use crate::types::api;
use crate::{errors, routes::SessionState, services, types::domain};

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub async fn rust_locker_migration(
    _state: SessionState,
    _merchant_id: &id_type::MerchantId,
) -> CustomResult<services::ApplicationResponse<MigrateCardResponse>, errors::ApiErrorResponse> {
    todo!()
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub async fn rust_locker_migration(
    state: SessionState,
    merchant_id: &id_type::MerchantId,
) -> CustomResult<services::ApplicationResponse<MigrateCardResponse>, errors::ApiErrorResponse> {
    use crate::db::customers::CustomerListConstraints;

    let db = state.store.as_ref();
    let key_manager_state = &(&state).into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_account = db
        .find_merchant_account_by_merchant_id(key_manager_state, merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    // Handle cases where the number of customers is greater than the limit
    let constraints = CustomerListConstraints {
        limit: u16::MAX,
        offset: None,
    };

    let domain_customers = db
        .list_customers_by_merchant_id(key_manager_state, merchant_id, &key_store, constraints)
        .await
        .change_context(errors::ApiErrorResponse::InternalServerError)?;

    let mut customers_moved = 0;
    let mut cards_moved = 0;

    for customer in domain_customers {
        let result = db
            .find_payment_method_by_customer_id_merchant_id_list(
                &customer.customer_id,
                merchant_id,
                None,
            )
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

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "payment_methods_v2")
))]
pub async fn call_to_locker(
    state: &SessionState,
    payment_methods: Vec<PaymentMethod>,
    customer_id: &id_type::CustomerId,
    merchant_id: &id_type::MerchantId,
    merchant_account: &domain::MerchantAccount,
) -> CustomResult<usize, errors::ApiErrorResponse> {
    let mut cards_moved = 0;

    for pm in payment_methods
        .into_iter()
        .filter(|pm| matches!(pm.payment_method, Some(storage_enums::PaymentMethod::Card)))
    {
        let card = cards::get_card_from_locker(
            state,
            customer_id,
            merchant_id,
            pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id),
        )
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
            #[cfg(feature = "payouts")]
            wallet: None,
            #[cfg(feature = "payouts")]
            bank_transfer: None,
            metadata: pm.metadata,
            customer_id: Some(pm.customer_id),
            card_network: card.card_brand,
            client_secret: None,
            payment_method_data: None,
            billing: None,
            connector_mandate_details: None,
            network_transaction_id: None,
        };

        let add_card_result = cards::add_card_hs(
                state,
                pm_create,
                &card_details,
                customer_id,
                merchant_account,
                api_enums::LockerChoice::HyperswitchCardVault,
                Some(pm.locker_id.as_ref().unwrap_or(&pm.payment_method_id)),

            )
            .await
            .change_context(errors::ApiErrorResponse::InternalServerError)
            .attach_printable(format!(
                "Card migration failed for merchant_id: {merchant_id:?}, customer_id: {customer_id:?}, payment_method_id: {} ",
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
                "Card migrated for merchant_id: {merchant_id:?}, customer_id: {customer_id:?}, payment_method_id: {} ",
                pm.payment_method_id
            );
    }

    Ok(cards_moved)
}

#[cfg(all(feature = "v2", feature = "payment_methods_v2"))]
pub async fn call_to_locker(
    _state: &SessionState,
    _payment_methods: Vec<PaymentMethod>,
    _customer_id: &id_type::CustomerId,
    _merchant_id: &id_type::MerchantId,
    _merchant_account: &domain::MerchantAccount,
) -> CustomResult<usize, errors::ApiErrorResponse> {
    todo!()
}
