use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::alert_rule_configurations::{
    AlertRuleConfigurationCreateRequest, AlertRuleConfigurationUpdateRequest,
};

use crate::{auth, core, services, state::AppState};

pub async fn create(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<AlertRuleConfigurationCreateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::alert_rule_configurations::create_alert_rule_configuration,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn update(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<String>,
    payload: web::Json<AlertRuleConfigurationUpdateRequest>,
) -> HttpResponse {
    let id = id.into_inner();
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        move |state, payload| {
            core::alert_rule_configurations::update_alert_rule_configuration(state, id, payload)
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::alert_rule_configurations::list_alert_rule_configurations,
        &auth::InternalApiKeyAuth,
    )
    .await
}
