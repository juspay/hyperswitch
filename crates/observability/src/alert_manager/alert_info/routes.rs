//! Handlers for the alert definition routes. The route tree that mounts them is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::alert_info::{
    AlertsInfoCreateRequest, AlertsInfoEnableRequest, AlertsInfoListRequest,
    AlertsInfoRetrieveRequest,
};

use crate::{auth, services, state::AppState};

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

pub async fn list(
    state: web::Data<AppState>,
    request: HttpRequest,
    query: web::Query<AlertsInfoListRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        query.into_inner(),
        crate::core::alert_manager::alert_info::core::list_alert_info,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        AlertsInfoRetrieveRequest {
            id: id.into_inner(),
        },
        crate::core::alert_manager::alert_info::core::retrieve_alert_info,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn enable(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
    payload: web::Json<AlertsInfoEnableRequest>,
) -> HttpResponse {
    let id = id.into_inner();
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (id, payload.into_inner()),
        |state, (id, payload)| {
            crate::core::alert_manager::alert_info::core::enable_alert_info(state, id, payload)
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn disable(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        AlertsInfoRetrieveRequest {
            id: id.into_inner(),
        },
        crate::core::alert_manager::alert_info::core::disable_alert_info,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        AlertsInfoRetrieveRequest {
            id: id.into_inner(),
        },
        crate::core::alert_manager::alert_info::core::delete_alert_info,
        &auth::InternalApiKeyAuth,
    )
    .await
}
