pub mod transformers;
pub mod utils;

use api_models::blocklist as api_blocklist;

use crate::{
    core::errors::{self, RouterResponse},
    routes::SessionState,
    services,
    types::domain,
};

pub async fn add_entry_to_blocklist(
    state: SessionState,
    platform: domain::Platform,
    body: api_blocklist::AddToBlocklistRequest,
) -> RouterResponse<api_blocklist::AddToBlocklistResponse> {
    utils::insert_entry_into_blocklist(&state, &platform, body)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn remove_entry_from_blocklist(
    state: SessionState,
    processor: domain::Processor,
    body: api_blocklist::DeleteFromBlocklistRequest,
) -> RouterResponse<api_blocklist::DeleteFromBlocklistResponse> {
    utils::delete_entry_from_blocklist(&state, processor.get_account().get_id(), body)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn list_blocklist_entries(
    state: SessionState,
    processor: domain::Processor,
    query: api_blocklist::ListBlocklistQuery,
) -> RouterResponse<api_blocklist::ListBlocklistResponse> {
    utils::list_blocklist_entries_for_merchant(&state, processor.get_account().get_id(), query)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn toggle_blocklist_guard(
    state: SessionState,
    processor: domain::Processor,
    query: api_blocklist::ToggleBlocklistQuery,
) -> RouterResponse<api_blocklist::ToggleBlocklistResponse> {
    utils::toggle_blocklist_guard_for_merchant(&state, processor.get_account().get_id(), query)
        .await
        .map(services::ApplicationResponse::Json)
}
