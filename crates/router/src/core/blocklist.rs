pub mod transformers;
pub mod utils;

use api_models::blocklist as api_blocklist;

use crate::{
    core::errors::{self, RouterResponse},
    routes::AppState,
    services,
    types::domain,
};

pub async fn add_entry_to_blocklist(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    body: api_blocklist::AddToBlocklistRequest,
) -> RouterResponse<api_blocklist::AddToBlocklistResponse> {
    utils::insert_entry_into_blocklist(&state, merchant_account.merchant_id, body)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn remove_entry_from_blocklist(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    body: api_blocklist::DeleteFromBlocklistRequest,
) -> RouterResponse<api_blocklist::DeleteFromBlocklistResponse> {
    utils::delete_entry_from_blocklist(&state, merchant_account.merchant_id, body)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn list_blocklist_entries(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    query: api_blocklist::ListBlocklistQuery,
) -> RouterResponse<Vec<api_blocklist::BlocklistResponse>> {
    utils::list_blocklist_entries_for_merchant(&state, merchant_account.merchant_id, query)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn toggle_blocklist_guard(
    state: AppState,
    merchant_account: domain::MerchantAccount,
    query: api_blocklist::ToggleBlocklistQuery,
) -> RouterResponse<api_blocklist::ToggleBlocklistResponse> {
    utils::toggle_blocklist_guard_for_merchant(&state, merchant_account.merchant_id, query)
        .await
        .map(services::ApplicationResponse::Json)
}
