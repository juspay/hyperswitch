use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, configs},
    services::{api, authentication as auth},
    types::api as api_types,
};

#[cfg(feature = "v1")]
const ADMIN_API_AUTH: auth::AdminApiAuth = auth::AdminApiAuth;
#[cfg(feature = "v2")]
const ADMIN_API_AUTH: auth::V2AdminApiAuth = auth::V2AdminApiAuth;

#[instrument(skip_all, fields(flow = ?Flow::CreateConfigKey))]
pub async fn config_key_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_types::Config>,
) -> impl Responder {
    let flow = Flow::CreateConfigKey;
    let payload = json_payload.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, data, _| configs::set_config(state, data),
        &ADMIN_API_AUTH,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::ConfigKeyFetch))]
pub async fn config_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::ConfigKeyFetch;
    let key = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        &key,
        |state, _, key, _| configs::read_config(state, key),
        &ADMIN_API_AUTH,
        api_locking::LockAction::NotApplicable,
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
    let flow = Flow::ConfigKeyUpdate;
    let mut payload = json_payload.into_inner();
    let key = path.into_inner();
    payload.key = key;

    api::server_wrap(
        flow,
        state,
        &req,
        &payload,
        |state, _, payload, _| configs::update_config(state, payload),
        &ADMIN_API_AUTH,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::ConfigKeyDelete))]
pub async fn config_key_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::ConfigKeyDelete;
    let key = path.into_inner();

    api::server_wrap(
        flow,
        state,
        &req,
        key,
        |state, _, key, _| configs::config_delete(state, key),
        &ADMIN_API_AUTH,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
