use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, unified_connector_service::kill_switch},
    services::{api as oss_api, authentication as auth},
};

/// Lists the scopes the UCS kill switch has currently tripped.
///
/// With over a thousand rollout keys provisioned, an on-call engineer cannot reconstruct this
/// from logs, so the switch is not operable without it.
#[instrument(skip_all, fields(flow = ?Flow::UnifiedConnectorServiceKillSwitchList))]
pub async fn list_kill_switch(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    let flow = Flow::UnifiedConnectorServiceKillSwitchList;

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _, (), _| kill_switch::list_tripped_scopes(state),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Clears a UCS kill switch trip, returning the scope to whatever its rollout config says.
/// `scope` is a value returned by the list endpoint.
#[instrument(skip_all, fields(flow = ?Flow::UnifiedConnectorServiceKillSwitchReset))]
pub async fn reset_kill_switch(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::UnifiedConnectorServiceKillSwitchReset;

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        path.into_inner(),
        |state, _, scope, _| kill_switch::reset(state, scope),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
