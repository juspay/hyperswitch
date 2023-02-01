use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::configs,
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::ConfigKeyFetch))]
pub async fn config_key_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let key = path.into_inner();

    api::server_wrap(
        &state,
        &req,
        &key,
        |state, _, key| configs::read_config(&*state.store, key),
        &auth::AdminApiAuth,
    )
    .await
}
