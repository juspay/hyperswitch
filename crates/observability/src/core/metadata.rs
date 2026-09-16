//! Per-request logic for per-alert metadata and snooze state.

use api_models::observability::alert_manager::metadata::{
    AlertMetadataEntryResponse, AlertMetadataListResponse, AlertMetadataPatchRequest,
    AlertMetadataPatchResponse,
};
use error_stack::ResultExt;

use crate::{
    domain_models::metadata::AlertMetadataPatch,
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn list(state: AppState, _: ()) -> ObservabilityApiResult<AlertMetadataListResponse> {
    let entries = state
        .store
        .list_alert_metadata()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list alert metadata")?;

    Ok(AlertMetadataListResponse {
        entries: entries
            .into_iter()
            .map(AlertMetadataEntryResponse::from)
            .collect(),
    })
}

pub async fn patch(
    state: AppState,
    (id, request): (String, AlertMetadataPatchRequest),
) -> ObservabilityApiResult<AlertMetadataPatchResponse> {
    let patch = AlertMetadataPatch::try_from_request(id, request)?;
    let entry = state
        .store
        .patch_alert_metadata(patch)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to patch alert metadata")?;

    Ok(AlertMetadataPatchResponse {
        ok: true,
        id: entry.id,
    })
}
