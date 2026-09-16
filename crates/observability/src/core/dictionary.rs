//! Per-request logic for alert dictionary state.

use api_models::observability::alert_manager::dictionary::{
    DictionaryEntryResponse, DictionaryListResponse, DictionaryUpsertRequest,
    DictionaryUpsertResponse,
};
use error_stack::ResultExt;

use crate::{
    domain_models::dictionary::DictionaryEntryNew,
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn list(state: AppState, _: ()) -> ObservabilityApiResult<DictionaryListResponse> {
    let entries = state
        .store
        .list_dictionary_entries()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list dictionary entries")?;

    Ok(DictionaryListResponse {
        entries: entries
            .into_iter()
            .map(DictionaryEntryResponse::from)
            .collect(),
    })
}

pub async fn upsert(
    state: AppState,
    request: DictionaryUpsertRequest,
) -> ObservabilityApiResult<DictionaryUpsertResponse> {
    let new = DictionaryEntryNew::try_from_request(request)?;
    let entry = state
        .store
        .upsert_dictionary_entry(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to upsert dictionary entry")?;

    Ok(DictionaryUpsertResponse {
        ok: true,
        name: entry.name,
        key_: entry.key_,
    })
}
