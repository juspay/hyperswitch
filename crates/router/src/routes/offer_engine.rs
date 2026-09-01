use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app;
use crate::{
    core::{api_locking, offer_engine},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::OfferEngineConnectivityCheck))]
pub async fn offer_engine_connectivity_check(
    state: web::Data<app::AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::OfferEngineConnectivityCheck;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _, _, _| offer_engine::connectivity::check_offer_engine_connectivity(state),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
