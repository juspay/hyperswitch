use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    alert_manager::{
        core,
        types::{mappers::MapperUpsertRequest, UserName},
    },
    auth, services,
    state::AppState,
};

pub async fn list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::mappers::list(state).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn read(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, key) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::mappers::read(state, &name, &key).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MapperUpsertRequest>,
) -> HttpResponse {
    let user = UserName::from_headers(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move { core::mappers::upsert(state, payload, user?).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn retire(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, key) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::mappers::retire(state, &name, &key).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}
