use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::{api_locking, cache},
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
        state,
        &req,
        &key,
        |state, _, key, _| cache::invalidate(state, key),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
