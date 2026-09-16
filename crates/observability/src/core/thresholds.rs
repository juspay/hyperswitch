//! Per-request logic for threshold overrides.

use api_models::observability::alert_manager::thresholds::{
    ThresholdDeleteRequest, ThresholdDeleteResponse, ThresholdListResponse, ThresholdResponse,
    ThresholdUpsertRequest, ThresholdUpsertResponse,
};
use error_stack::{report, ResultExt};

use crate::{
    domain_models::thresholds::{ThresholdOverrideNew, ThresholdUpsertOutcome},
    errors::{ObservabilityApiResult, ObservabilityError},
    state::AppState,
};

pub async fn list(state: AppState, _: ()) -> ObservabilityApiResult<ThresholdListResponse> {
    let overrides = state
        .store
        .list_threshold_overrides()
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list threshold overrides")?;

    Ok(ThresholdListResponse {
        overrides: overrides.into_iter().map(ThresholdResponse::from).collect(),
    })
}

pub async fn upsert(
    state: AppState,
    request: ThresholdUpsertRequest,
) -> ObservabilityApiResult<ThresholdUpsertResponse> {
    let new = ThresholdOverrideNew::try_from(request)?;
    match state
        .store
        .upsert_threshold_override(new, state.conf.limits.max_active_threshold_rules)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to upsert threshold override")?
    {
        ThresholdUpsertOutcome::Stored(threshold) => Ok(ThresholdUpsertResponse {
            threshold: ThresholdResponse::from(threshold),
        }),
        ThresholdUpsertOutcome::ActiveRuleLimitReached => {
            Err(report!(ObservabilityError::ActiveRuleLimitReached))
        }
    }
}

pub async fn delete(
    state: AppState,
    request: ThresholdDeleteRequest,
) -> ObservabilityApiResult<ThresholdDeleteResponse> {
    let tombstone = ThresholdOverrideNew::try_from(request)?;
    state
        .store
        .delete_threshold_override(tombstone)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to tombstone threshold override")?;

    Ok(ThresholdDeleteResponse { status: "deleted" })
}
