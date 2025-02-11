use actix_web::{web, HttpRequest, HttpResponse};
use common_utils::errors::CustomResult;
use hyperswitch_domain_models::merchant_key_store::MerchantKeyStore;
use router_env::Flow;

use super::{app::SessionState, AppState};
use crate::{
    core::{
        api_locking,
        errors::{self, utils::StorageErrorExt},
        payment_method_billing_address_migration,
    },
    services::{api, authentication as auth},
    types::domain,
};

pub async fn payment_method_billing_address_migration(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> HttpResponse {
    let flow = Flow::PaymentMethodBillingAddressMigration;
    let (merchant_id, payment_method_id) = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _, _, _| {
            let merchant_id = merchant_id.clone();
            let payment_method_id = payment_method_id.clone();
            async move {
                let (key_store, merchant_account) =
                    get_merchant_account(&state, &merchant_id).await?;
                    Box::pin(payment_method_billing_address_migration::payment_method_billing_address_migration(state, &merchant_account, &merchant_id, &payment_method_id, &key_store)).await
            }
        },
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

async fn get_merchant_account(
    state: &SessionState,
    merchant_id: &common_utils::id_type::MerchantId,
) -> CustomResult<(MerchantKeyStore, domain::MerchantAccount), errors::ApiErrorResponse> {
    let key_manager_state = &state.into();
    let key_store = state
        .store
        .get_merchant_key_store_by_merchant_id(
            key_manager_state,
            merchant_id,
            &state.store.get_master_key().to_vec().into(),
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;

    let merchant_account = state
        .store
        .find_merchant_account_by_merchant_id(key_manager_state, merchant_id, &key_store)
        .await
        .to_not_found_response(errors::ApiErrorResponse::MerchantAccountNotFound)?;
    Ok((key_store, merchant_account))
}
