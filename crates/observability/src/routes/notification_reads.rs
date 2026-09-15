use actix_web::{web, HttpRequest, HttpResponse};
use api_models::observability::notification_reads::{
    NotificationReadsRetrieveRequest, NotificationReadsUpsertRequest,
};

use crate::{auth, core, services, state::AppState};

pub async fn retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
    query: web::Query<NotificationReadsRetrieveRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        query.into_inner(),
        core::notification_reads::retrieve_notification_read,
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<NotificationReadsUpsertRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        core::notification_reads::upsert_notification_read,
        &auth::InternalApiKeyAuth,
    )
    .await
}
