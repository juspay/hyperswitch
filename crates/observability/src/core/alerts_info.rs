//! Per-request logic for alert definitions.

use api_models::observability::alert_manager::alerts_info::{
    AlertsInfoCreateRequest, AlertsInfoResponse,
};
use error_stack::ResultExt;

use crate::{
    domain_models::alerts_info::AlertsInfoNew,
    errors::{ObservabilityApiResult, ObservabilityError},
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
