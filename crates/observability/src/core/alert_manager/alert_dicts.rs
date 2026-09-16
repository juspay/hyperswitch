//! Per-request logic for the mappers dictionary.

use api_models::observability::alert_manager::alert_dicts::{
    AlertsDictsCreateRequest, AlertsDictsDeleteRequest, AlertsDictsDeleteResponse,
    AlertsDictsListRequest, AlertsDictsListResponse, AlertsDictsResponse,
    AlertsDictsRetrieveRequest,
};
use error_stack::ResultExt;

use crate::{
    domain_models::alert_manager::alert_dicts::{
        parse_alert_dict_id, AlertsDictsFilter, AlertsDictsNew,
    },
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
};

/// Save a new version of a dictionary entry.
pub async fn create_alert_dict(
    state: AppState,
    request: AlertsDictsCreateRequest,
) -> ObservabilityApiResult<AlertsDictsResponse> {
    let new = AlertsDictsNew::try_from(request)?;

    let stored = state
        .store
        .insert_alert_dict_version(new)
        .await
        .to_duplicate_response(ObservabilityError::DuplicateResource)
        .attach_printable("Failed to save an alerts_dicts version")?;

    Ok(AlertsDictsResponse::from(stored))
}

/// List dictionary entries matching the given filter.
pub async fn list_alert_dicts(
    state: AppState,
    request: AlertsDictsListRequest,
) -> ObservabilityApiResult<AlertsDictsListResponse> {
    let filter = AlertsDictsFilter::from(request);

    let entries = state
        .store
        .list_alert_dicts_by_filter(filter)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list alerts_dicts")?;

    let data: Vec<AlertsDictsResponse> =
        entries.into_iter().map(AlertsDictsResponse::from).collect();

    Ok(AlertsDictsListResponse {
        count: data.len(),
        data,
    })
}

/// Retrieve one dictionary entry by id, any version.
pub async fn retrieve_alert_dict(
    state: AppState,
    request: AlertsDictsRetrieveRequest,
) -> ObservabilityApiResult<AlertsDictsResponse> {
    let id = parse_alert_dict_id(&request.id)?;

    let entry = state
        .store
        .find_alert_dict_by_id(id)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(AlertsDictsResponse::from(entry))
}

/// Delete one dictionary entry by id, any version. Never re-enables another version.
pub async fn delete_alert_dict(
    state: AppState,
    request: AlertsDictsDeleteRequest,
) -> ObservabilityApiResult<AlertsDictsDeleteResponse> {
    let id = parse_alert_dict_id(&request.id)?;

    let deleted = state
        .store
        .delete_alert_dict_by_id(id)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(AlertsDictsDeleteResponse {
        id: request.id,
        deleted,
    })
}
