pub mod utils;

use api_models::card_testing_guard as api_card_testing_guard;

use crate::{
    core::errors::{self, RouterResponse},
    routes::SessionState,
    services,
    types::domain,
};

pub async fn update_card_testing_guard(
    state: SessionState,
    merchant_account: domain::MerchantAccount,
    payload: api_card_testing_guard::UpdateCardTestingGuardRequest,
) -> RouterResponse<api_card_testing_guard::UpdateCardTestingGuardResponse> {
    utils::update_card_testing_guard_for_merchant(&state, merchant_account.get_id(), payload)
        .await
        .map(services::ApplicationResponse::Json)
}
