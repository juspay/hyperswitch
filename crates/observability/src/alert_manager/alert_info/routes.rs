//! Handlers for the alert definition routes. The route tree that mounts them is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::alert_info::AlertsInfoCreateRequest;

use crate::{auth, core, services, state::AppState};

/// `POST /alerts/alerts_manager/info`.
pub async fn create(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<AlertsInfoCreateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        crate::core::alert_manager::alert_info::core::create_alert_info,
        &auth::InternalApiKeyAuth,
    )
    .await
}
