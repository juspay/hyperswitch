use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{api_locking, proxy},
    services::{api, authentication as auth},
    types::domain,
};

#[instrument(skip_all, fields(flow = ?Flow::Proxy))]
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
            let platform = auth.into();
            proxy::proxy_core(state, platform, req)
        },
        &auth::V2ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
