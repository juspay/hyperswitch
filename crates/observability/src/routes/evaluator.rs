use actix_web::{web, HttpRequest, HttpResponse};

use crate::{auth, core, services, state::AppState};

pub async fn notify(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::evaluator::notify,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn evaluate(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        core::evaluator::evaluate,
        &auth::InternalApiKeyAuth,
    )
    .await
}
