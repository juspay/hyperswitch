use actix_web::{web, HttpRequest, HttpResponse};

use crate::{auth, core, services, state::AppState, types::mappers::MapperEntrySaveRequest};

pub async fn list_mappers(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::mappers::list_mappers(state).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn read_mapper(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, key) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::mappers::read_mapper(state, name, key).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn save_mapper(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<MapperEntrySaveRequest>,
) -> HttpResponse {
    let user_name = auth::get_user_name(request.headers());

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::mappers::save_mapper(state, payload, user_name?).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn delete_mapper(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, key) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::mappers::delete_mapper(state, name, key).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}
