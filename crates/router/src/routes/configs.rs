use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::configs,
    services::{api, authentication as auth},
    types::api as api_types,
};

#[instrument(skip_all, fields(flow = ?Flow::ConfigKeyFetch))]
pub async fn config_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let key = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        &key,
        |state, _, key| configs::read_config(&*state.store, key),
        &auth::AdminApiAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ConfigKeyUpdate))]
pub async fn config_key_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<api_types::ConfigUpdate>,
) -> impl Responder {
    let mut payload = json_payload.into_inner();
    let key = path.into_inner();
    payload.key = key;

    api::server_wrap(
        state.get_ref(),
        &req,
        &payload,
        |state, _, payload| configs::update_config(&*state.store, payload),
        &auth::AdminApiAuth,
    )
    .await
}
