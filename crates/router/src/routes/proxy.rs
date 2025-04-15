use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{api_locking, proxy},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::Proxy))]
#[cfg(feature = "oltp")]
pub async fn proxy(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<api_models::proxy::ProxyRequest>,
) -> impl Responder {
    let flow = Flow::Proxy;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            proxy::proxy_core(
                state,
                auth.merchant_account,
                auth.profile,
                auth.key_store,
                req,
            )
        },
        &auth::V2ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}