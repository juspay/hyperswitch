use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, configs},
    services::{api, authentication as auth},
    types::api as api_types,
};

#[instrument(skip_all, fields(flow = ?Flow::CreateConfigKey))]
/// Asynchronously creates a new configuration key using the provided JSON payload and the application state.
/// 
/// # Arguments
/// * `state` - The application state data
/// * `req` - The HTTP request
/// * `json_payload` - The JSON payload containing the configuration data
/// 
/// # Returns
/// 
/// A future that resolves to a responder
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
        |state, _, data| configs::set_config(state, data),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::ConfigKeyFetch))]
/// Retrieves a configuration key from the AppState using the provided key. 
/// It uses the api::server_wrap method to handle the authentication, authorization, and locking logic before calling the configs::read_config method to retrieve the configuration from the state.
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
        |state, _, key| configs::read_config(state, key),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::ConfigKeyUpdate))]
/// Handles the update of a configuration key by receiving the AppState, HttpRequest, the key path, and a JSON payload containing the configuration update. It then wraps the update operation in the server_wrap function along with the Flow type, state, request, payload, update_config function, admin authentication, and locking action. It returns a Responder representing the result of the update operation.
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
        |state, _, payload| configs::update_config(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
