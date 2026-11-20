use actix_web::{web, HttpRequest, HttpResponse};
use common_types::alert_rules::MetricSource;

use crate::{auth, core, services, state::AppState};

pub async fn notify(
    state: web::Data<AppState>,
    request: HttpRequest,
    source: MetricSource,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        source,
        core::alert_evaluation::notify,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn evaluate(
    state: web::Data<AppState>,
    request: HttpRequest,
    source: MetricSource,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        source,
        core::alert_evaluation::evaluate,
        &auth::InternalApiKeyAuth,
    )
    .await
}
