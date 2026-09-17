//! Per-request logic for alert lifecycle episode state.

use api_models::observability::alert_manager::lifecycle_events::{
    LifecycleEventResponse, LifecycleEventsBatchRequest, LifecycleEventsBatchResponse,
    LifecycleEventsListResponse, LifecycleEventsQuery,
};
use error_stack::ResultExt;

use crate::{
    domain_models::lifecycle_events::LifecycleEventsBatch,
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn list(
    state: AppState,
    query: LifecycleEventsQuery,
) -> ObservabilityApiResult<LifecycleEventsListResponse> {
    let events = state
        .store
        .list_lifecycle_events(query.from, query.to)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list alert lifecycle events")?;

    Ok(LifecycleEventsListResponse {
        events: events
            .into_iter()
            .map(LifecycleEventResponse::from)
            .collect(),
        server_time: common_utils::date_time::now(),
    })
}

pub async fn replace_batch(
    state: AppState,
    request: LifecycleEventsBatchRequest,
) -> ObservabilityApiResult<LifecycleEventsBatchResponse> {
    let batch = LifecycleEventsBatch::try_from_request(request, common_utils::date_time::now())?;
    let persisted = state
        .store
        .replace_lifecycle_events(batch)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to persist alert lifecycle events")?;

    Ok(LifecycleEventsBatchResponse {
        ok: true,
        persisted,
    })
}
