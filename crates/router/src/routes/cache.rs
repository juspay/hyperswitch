use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::cache,
    services::{api, authentication as auth},
};

#[instrument(skip_all)]
pub async fn invalidate(
    state: web::Data<AppState>,
    req: HttpRequest,
    key: web::Path<String>,
) -> impl Responder {
    let flow = Flow::CacheInvalidate;

    let key = key.into_inner().to_owned();

    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        &key,
        |state, _, key| cache::invalidate(&*state.store, key),
        &auth::AdminApiAuth,
    )
    .await
}
