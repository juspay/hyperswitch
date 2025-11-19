use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{api_locking, relay},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::Relay))]
#[cfg(feature = "oltp")]
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
            let platform = auth.clone().into();
            relay::relay_flow_decider(
                state,
                platform,
                #[cfg(feature = "v1")]
                auth.profile_id,
                #[cfg(feature = "v2")]
                Some(auth.profile.get_id().clone()),
                req,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RelayRetrieve))]
#[cfg(feature = "oltp")]
pub async fn relay_retrieve(
    state: web::Data<app::AppState>,
    path: web::Path<common_utils::id_type::RelayId>,
    req: actix_web::HttpRequest,
    query_params: web::Query<api_models::relay::RelayRetrieveBody>,
) -> impl Responder {
    let flow = Flow::RelayRetrieve;
    let relay_retrieve_request = api_models::relay::RelayRetrieveRequest {
        force_sync: query_params.force_sync,
        id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        relay_retrieve_request,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.clone().into();
            relay::relay_retrieve(
                state,
                platform,
                #[cfg(feature = "v1")]
                auth.profile_id,
                #[cfg(feature = "v2")]
                Some(auth.profile.get_id().clone()),
                req,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
