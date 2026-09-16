//! Handlers for the mappers dictionary routes. The route tree that mounts them is in
//! [`crate::routes::app`].

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::alert_dicts::{
    AlertsDictsCreateRequest, AlertsDictsDeleteRequest, AlertsDictsListRequest,
    AlertsDictsRetrieveRequest,
};

use crate::{auth, services, state::AppState};

/// `POST /alerts/alerts_manager/dicts`.
pub async fn create(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<AlertsDictsCreateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        crate::core::alert_manager::alert_dicts::create_alert_dict,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/alerts_manager/dicts`.
pub async fn list(
    state: web::Data<AppState>,
    request: HttpRequest,
    query: web::Query<AlertsDictsListRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        query.into_inner(),
        crate::core::alert_manager::alert_dicts::list_alert_dicts,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `GET /alerts/alerts_manager/dicts/{id}`.
pub async fn retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        AlertsDictsRetrieveRequest {
            id: id.into_inner(),
        },
        crate::core::alert_manager::alert_dicts::retrieve_alert_dict,
        &auth::InternalApiKeyAuth,
    )
    .await
}

/// `DELETE /alerts/alerts_manager/dicts/{id}`.
pub async fn delete(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        AlertsDictsDeleteRequest {
            id: id.into_inner(),
        },
        crate::core::alert_manager::alert_dicts::delete_alert_dict,
        &auth::InternalApiKeyAuth,
    )
    .await
}
