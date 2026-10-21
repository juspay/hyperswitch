use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::alert_rules::{
    AlertRuleCreateRequest, AlertRuleUpdateRequest,
};

use crate::{auth, core, services, state::AppState};

pub async fn create(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<AlertRuleCreateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_rules::create_alert_rule,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn update(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
    payload: web::Json<AlertRuleUpdateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (id.into_inner(), payload.into_inner()),
        core::alert_rules::update_alert_rule,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::alert_rules::list_alert_rules,
        &auth::InternalApiKeyAuth,
    )
    .await
}
