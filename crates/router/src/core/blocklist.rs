pub mod batch;
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
    profile_id: Option<common_utils::id_type::ProfileId>,
    body: api_blocklist::AddToBlocklistRequest,
) -> RouterResponse<api_blocklist::AddToBlocklistResponse> {
    utils::insert_entry_into_blocklist(&state, &platform, profile_id, body)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn remove_entry_from_blocklist(
    state: SessionState,
    processor: domain::Processor,
    profile_id: Option<common_utils::id_type::ProfileId>,
    body: api_blocklist::DeleteFromBlocklistRequest,
) -> RouterResponse<api_blocklist::DeleteFromBlocklistResponse> {
    utils::delete_entry_from_blocklist(&state, &processor, profile_id, body)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn list_blocklist_entries(
    state: SessionState,
    processor: domain::Processor,
    profile_id: Option<common_utils::id_type::ProfileId>,
    query: api_blocklist::ListBlocklistQuery,
) -> RouterResponse<api_blocklist::ListBlocklistResponse> {
    utils::list_blocklist_entries_for_merchant(
        &state,
        processor.get_account().get_id(),
        profile_id.as_ref(),
        query,
    )
    .await
    .map(services::ApplicationResponse::Json)
}

pub async fn get_blocklist_count(
    state: SessionState,
    processor: domain::Processor,
    profile_id: Option<common_utils::id_type::ProfileId>,
    query: api_blocklist::BlocklistCountQuery,
) -> RouterResponse<api_blocklist::BlocklistCountResponse> {
    utils::get_blocklist_count(&state, &processor, profile_id, query)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn lookup_blocklist_entry(
    state: SessionState,
    processor: domain::Processor,
    profile_id: Option<common_utils::id_type::ProfileId>,
    query: api_blocklist::BlocklistLookupQuery,
) -> RouterResponse<api_blocklist::BlocklistLookupResponse> {
    utils::lookup_blocklist_entry(&state, &processor, profile_id, query)
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

pub async fn upload_batch_blocklist(
    state: SessionState,
    platform: domain::Platform,
    profile_id: Option<common_utils::id_type::ProfileId>,
    csv_bytes: bytes::Bytes,
) -> RouterResponse<api_blocklist::BatchBlocklistUploadResponse> {
    batch::initiate_batch_blocklist_upload(&state, &platform, profile_id, csv_bytes)
        .await
        .map(services::ApplicationResponse::Json)
}

pub async fn get_batch_blocklist_job_status(
    state: SessionState,
    platform: domain::Platform,
    job_id: String,
) -> RouterResponse<api_blocklist::BatchBlocklistJobStatusResponse> {
    batch::get_batch_blocklist_job_status(
        &state,
        platform.get_processor().get_account().get_id(),
        &job_id,
    )
    .await
    .map(services::ApplicationResponse::Json)
}

pub async fn list_batch_blocklist_jobs(
    state: SessionState,
    platform: domain::Platform,
    query: api_blocklist::ListBatchBlocklistJobsQuery,
) -> RouterResponse<api_blocklist::ListBatchBlocklistJobsResponse> {
    batch::list_batch_blocklist_jobs(
        &state,
        platform.get_processor().get_account().get_id(),
        query,
    )
    .await
    .map(services::ApplicationResponse::Json)
}
