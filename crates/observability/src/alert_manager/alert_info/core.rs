//! Per-request logic for alert definitions.

use api_models::observability::alert_manager::alert_info::{
    AlertsInfoCreateRequest, AlertsInfoDeleteResponse, AlertsInfoEnableRequest,
    AlertsInfoListRequest, AlertsInfoListResponse, AlertsInfoResponse, AlertsInfoRetrieveRequest,
};
use error_stack::ResultExt;

use crate::{
    domain_models::alert_manager::alert_info::domain_models::AlertsInfoNew,
    errors::{ObservabilityApiResult, ObservabilityError, StorageErrorExt},
    state::AppState,
};

/// Store a new alert definition.
pub async fn create_alert_info(
    state: AppState,
    request: AlertsInfoCreateRequest,
) -> ObservabilityApiResult<AlertsInfoResponse> {
    let new = AlertsInfoNew::try_from(request)?;

    let stored = state
        .store
        .insert_alert_info(new)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to insert into alerts_info")?;

    Ok(AlertsInfoResponse::from(stored))
}

pub async fn list_alert_info(
    state: AppState,
    request: AlertsInfoListRequest,
) -> ObservabilityApiResult<AlertsInfoListResponse> {
    let entries = state
        .store
        .list_alert_info(request.name, request.product, request.is_enabled)
        .await
        .change_context(ObservabilityError::InternalServerError)
        .attach_printable("Failed to list alerts_info")?;

    let data = entries
        .into_iter()
        .map(AlertsInfoResponse::from)
        .collect::<Vec<_>>();
    Ok(AlertsInfoListResponse {
        count: data.len(),
        data,
    })
}

pub async fn retrieve_alert_info(
    state: AppState,
    request: AlertsInfoRetrieveRequest,
) -> ObservabilityApiResult<AlertsInfoResponse> {
    let entry = state
        .store
        .find_alert_info_by_id(request.id)
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(AlertsInfoResponse::from(entry))
}

pub async fn enable_alert_info(
    state: AppState,
    id: String,
    request: AlertsInfoEnableRequest,
) -> ObservabilityApiResult<AlertsInfoResponse> {
    let current = state
        .store
        .find_alert_info_by_id(id.clone())
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    if current.author.as_deref() == Some(request.approver.as_str()) {
        Err(error_stack::report!(ObservabilityError::InvalidRequest))
            .attach_printable("Approver cannot be the same as the author")?
    }

    let updated = state
        .store
        .update_alert_info_by_id(
            id,
            diesel_models::observability::alert_manager::alert_info::AlertsInfoUpdate {
                is_enabled: Some(true),
                approver: Some(request.approver),
                author: None,
                last_updated_at: Some(common_utils::date_time::now()),
            },
        )
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(AlertsInfoResponse::from(updated))
}

pub async fn disable_alert_info(
    state: AppState,
    request: AlertsInfoRetrieveRequest,
) -> ObservabilityApiResult<AlertsInfoResponse> {
    let updated = state
        .store
        .update_alert_info_by_id(
            request.id,
            diesel_models::observability::alert_manager::alert_info::AlertsInfoUpdate {
                is_enabled: Some(false),
                approver: None,
                author: Some("unified_alerts".to_owned()),
                last_updated_at: Some(common_utils::date_time::now()),
            },
        )
        .await
        .to_not_found_response(ObservabilityError::ResourceNotFound)?;

    Ok(AlertsInfoResponse::from(updated))
}

pub async fn delete_alert_info(
    state: AppState,
    request: AlertsInfoRetrieveRequest,
) -> ObservabilityApiResult<AlertsInfoDeleteResponse> {
    let deleted = state
        .store
        .delete_alert_info_by_id(request.id.clone())
        .await
        .change_context(ObservabilityError::InternalServerError)?;

    if !deleted {
        Err(error_stack::report!(ObservabilityError::ResourceNotFound))?
    }

    Ok(AlertsInfoDeleteResponse {
        id: request.id,
        deleted,
    })
}
