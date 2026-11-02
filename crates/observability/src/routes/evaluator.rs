use actix_web::{web, HttpRequest, HttpResponse};

use crate::{auth, core, services, state::AppState};

pub async fn notify(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::alert_evaluation::notify,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn notify_loki(
    state: web::Data<AppState>,
    request: HttpRequest,
    body: web::Json<core::alert_evaluation::grafana_loki::ApplicationNotifyRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        body.into_inner(),
        core::alert_evaluation::grafana_loki::notify,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn evaluate_loki(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::alert_evaluation::grafana_loki::evaluate,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn evaluate(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::alert_evaluation::evaluate,
        &auth::InternalApiKeyAuth,
    )
    .await
}
