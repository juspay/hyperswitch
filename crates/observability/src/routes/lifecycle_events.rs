//! Authenticated alert lifecycle episode handlers.

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::alert_manager::lifecycle_events::{
    LifecycleEventsBatchRequest, LifecycleEventsQuery,
};

use crate::{auth, core, services, state::AppState};

pub async fn list(
    state: web::Data<AppState>,
    request: HttpRequest,
    query: web::Query<LifecycleEventsQuery>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        query.into_inner(),
        core::lifecycle_events::list,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn replace_batch(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<LifecycleEventsBatchRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::lifecycle_events::replace_batch,
        &auth::InternalApiKeyAuth,
    )
    .await
}
