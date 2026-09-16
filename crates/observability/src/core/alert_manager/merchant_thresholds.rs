//! Per-request logic for per-merchant threshold overrides.

use api_models::observability::alert_manager::merchant_thresholds::{
    MerchantThresholdsDeleteByFilterRequest, MerchantThresholdsDeleteRequest,
    MerchantThresholdsListRequest, MerchantThresholdsListResponse, MerchantThresholdsResponse,
    MerchantThresholdsUpdateRequest, MerchantThresholdsUpsertRequest,
};
use error_stack::ResultExt;

use crate::{
    domain_models::alert_manager::merchant_thresholds::{
        list_response, parse_id, MerchantThresholdsBulkUpdate, MerchantThresholdsFilter,
        MerchantThresholdsKeyFilter, MerchantThresholdsNew,
    },
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
};

/// List rows matching the given filter.
pub async fn list_merchant_thresholds(
    state: AppState,
    request: MerchantThresholdsListRequest,
) -> ObservabilityApiResult<MerchantThresholdsListResponse> {
    let filter = MerchantThresholdsFilter::try_from(request)?;

    let rows = state
        .store
        .list_merchant_thresholds_by_filter(filter)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list merchant_thresholds")?;

    Ok(list_response(rows))
}

/// Insert a new override, or, on a conflict of `(name, product, merchant_id, profile_id,
/// is_enabled, author)`, set only the non-key columns actually sent.
pub async fn upsert_merchant_threshold(
    state: AppState,
    request: MerchantThresholdsUpsertRequest,
) -> ObservabilityApiResult<MerchantThresholdsListResponse> {
    let new = MerchantThresholdsNew::try_from(request)?;

    let rows = state
        .store
        .upsert_merchant_threshold(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to upsert into merchant_thresholds")?;

    Ok(list_response(rows))
}

/// Set or clear columns on every row matching the keys sent.
pub async fn update_merchant_thresholds(
    state: AppState,
    request: MerchantThresholdsUpdateRequest,
) -> ObservabilityApiResult<MerchantThresholdsListResponse> {
    let MerchantThresholdsBulkUpdate { filter, update } =
        MerchantThresholdsBulkUpdate::try_from(request)?;

    let rows = state
        .store
        .update_merchant_thresholds_by_filter(filter, update)
        .await
        .to_duplicate_response(ObservabilityError::DuplicateResource)
        .attach_printable("Failed to update merchant_thresholds")?;

    Ok(list_response(rows))
}

/// Delete one override by id.
pub async fn delete_merchant_threshold(
    state: AppState,
    request: MerchantThresholdsDeleteRequest,
) -> ObservabilityApiResult<MerchantThresholdsResponse> {
    let id = parse_id(&request.id)?;

    let row = state
        .store
        .delete_merchant_threshold_by_id(id)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(MerchantThresholdsResponse::from(row))
}

/// Delete every row matching `name`, `product` and, when sent, `merchant_id` and `profile_id`.
pub async fn delete_merchant_thresholds_by_filter(
    state: AppState,
    request: MerchantThresholdsDeleteByFilterRequest,
) -> ObservabilityApiResult<MerchantThresholdsListResponse> {
    let filter = MerchantThresholdsKeyFilter::from(request);

    let rows = state
        .store
        .delete_merchant_thresholds_by_filter(filter)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to delete from merchant_thresholds")?;

    Ok(list_response(rows))
}
