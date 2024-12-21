use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{api_locking, relay},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::Relay))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn relay(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<api_models::relay::RelayRequest>,
) -> impl Responder {
    let flow = Flow::Relay;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            relay::relay(
                state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                req,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
