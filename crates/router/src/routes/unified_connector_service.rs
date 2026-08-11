use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, unified_connector_service::kill_switch},
    services::{api as oss_api, authentication as auth},
};

/// Lists the scopes the UCS kill switch has currently cut over.
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
        |state, _, (), _| kill_switch::list_cut_over_scopes(state),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Clears the UCS kill switch cutover for a merchant, connector and flow, returning the scope to
/// whatever its `ucs_rollout_config` key says.
#[instrument(skip_all, fields(flow = ?Flow::UnifiedConnectorServiceKillSwitchReset))]
pub async fn reset_kill_switch(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String, String)>,
) -> impl Responder {
    let flow = Flow::UnifiedConnectorServiceKillSwitchReset;
    let (merchant_id, connector, flow_name) = path.into_inner();
    let payload = kill_switch::KillSwitchScopeRequest {
        merchant_id,
        connector,
        flow: flow_name,
    };

    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, payload, _| kill_switch::reset_cut_over(state, payload),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
