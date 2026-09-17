//! Authenticated alert rule-toggle handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::rule_toggles::RuleToggleSetRequest;

use crate::{auth, core, services, state::AppState};

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::rule_toggles::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn set(
    state: web::Data<AppState>,
    request: HttpRequest,
    rule_id: web::Path<String>,
    payload: web::Json<RuleToggleSetRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (rule_id.into_inner(), payload.into_inner()),
        core::rule_toggles::set,
        &auth::InternalApiKeyAuth,
    )
    .await
}
