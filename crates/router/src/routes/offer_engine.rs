use actix_web::{web, HttpRequest, HttpResponse};
use api_models::offer_engine as offer_engine_api;
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

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::OfferEngineBrowseOffers))]
pub async fn offer_engine_browse_offers(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    json_payload: web::Json<offer_engine_api::BrowseOffersRequest>,
) -> HttpResponse {
    let flow = Flow::OfferEngineBrowseOffers;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, payload, _| {
            let profile_id = auth.profile.map(|profile| profile.get_id().clone());
            offer_engine::browse::browse_offers(state, auth.platform, profile_id, payload)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
