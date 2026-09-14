use actix_web::{web, HttpRequest, HttpResponse};

use crate::{
    auth, core, services,
    state::AppState,
    types::config::{
        AlertDefinitionCreateRequest, AlertDefinitionUpdateRequest, AlertEnablementUpsertRequest,
    },
};

pub async fn definition_list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::list_definitions(state).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn definition_create(
    state: web::Data<AppState>,
    request: HttpRequest,
    payload: web::Json<AlertDefinitionCreateRequest>,
) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move { core::config::create_definition(state, payload).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn definition_retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<uuid::Uuid>,
) -> HttpResponse {
    let id = id.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::retrieve_definition(state, id).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn definition_update(
    state: web::Data<AppState>,
    request: HttpRequest,
    id: web::Path<uuid::Uuid>,
    payload: web::Json<AlertDefinitionUpdateRequest>,
) -> HttpResponse {
    let id = id.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move { core::config::update_definition(state, id, payload).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn enablement_list(state: web::Data<AppState>, request: HttpRequest) -> HttpResponse {
    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::list_enablements(state).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn enablement_retrieve(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (name, product) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        (),
        |state, ()| async move { core::config::retrieve_enablement(state, name, product).await },
        &auth::InternalApiKeyAuth,
    )
    .await
}

pub async fn enablement_upsert(
    state: web::Data<AppState>,
    request: HttpRequest,
    path: web::Path<(String, String)>,
    payload: web::Json<AlertEnablementUpsertRequest>,
) -> HttpResponse {
    let (name, product) = path.into_inner();

    services::server_wrap(
        state.get_ref().clone(),
        &request,
        payload.into_inner(),
        |state, payload| async move {
            core::config::upsert_enablement(state, name, product, payload).await
        },
        &auth::InternalApiKeyAuth,
    )
    .await
}
