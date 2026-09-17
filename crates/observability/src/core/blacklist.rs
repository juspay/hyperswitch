//! Per-request logic for alert blacklist state.

use api_models::observability::alert_manager::blacklist::{
    BlacklistDeleteRequest, BlacklistDeleteResponse, BlacklistEntry, BlacklistListResponse,
    BlacklistUpsertRequest, BlacklistUpsertResponse,
};
use error_stack::{report, ResultExt};

use crate::{
    domain_models::blacklist::{BlacklistEntryNew, BlacklistUpsertOutcome},
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

const MAX_ACTIVE_BLACKLIST_RULES: i64 = 5000;

pub async fn list(state: AppState, _: ()) -> ObservabilityApiResult<BlacklistListResponse> {
    let entries = state
        .store
        .list_blacklist_entries()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list blacklist entries")?;
    Ok(BlacklistListResponse {
        entries: entries.into_iter().map(BlacklistEntry::from).collect(),
    })
}

pub async fn upsert(
    state: AppState,
    request: BlacklistUpsertRequest,
) -> ObservabilityApiResult<BlacklistUpsertResponse> {
    let new = BlacklistEntryNew::try_from(request)?;
    match state
        .store
        .upsert_blacklist_entry(new, MAX_ACTIVE_BLACKLIST_RULES)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to upsert blacklist entry")?
    {
        BlacklistUpsertOutcome::Stored(entry) => Ok(BlacklistUpsertResponse {
            status: "SUCCESS",
            rule_id: entry.rule_id,
            merchant_id: entry.merchant_id,
        }),
        BlacklistUpsertOutcome::ActiveRuleLimitReached => {
            Err(report!(ObservabilityError::BlacklistActiveRuleLimitReached))
        }
    }
}

pub async fn delete(
    state: AppState,
    request: BlacklistDeleteRequest,
) -> ObservabilityApiResult<BlacklistDeleteResponse> {
    let tombstone = BlacklistEntryNew::try_from(request)?;
    state
        .store
        .delete_blacklist_entry(tombstone)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to tombstone blacklist entry")?;
    Ok(BlacklistDeleteResponse { status: "SUCCESS" })
}
