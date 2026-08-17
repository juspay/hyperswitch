use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, unified_connector_service::kill_switch},
    services::{api as oss_api, authentication as auth},
};

/// Reports whether the UCS kill switch has tripped a scope, and what tripped it.
///
/// `scope` is the `rollout_scope` the trip logs, or the whole redis key.
#[instrument(skip_all, fields(flow = ?Flow::UnifiedConnectorServiceKillSwitchStatus))]
pub async fn kill_switch_status(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::UnifiedConnectorServiceKillSwitchStatus;

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        path.into_inner(),
        |state, _, scope, _| kill_switch::trip_status(state, scope),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Clears a UCS kill switch trip, returning the scope to whatever its rollout config says.
/// Takes the same `scope` as the status endpoint.
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
