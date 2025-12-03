use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{api_locking, three_ds_decision_rule as three_ds_decision_rule_core},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::ThreeDsDecisionRuleExecute))]
#[cfg(feature = "oltp")]
pub async fn execute_decision_rule(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest>,
) -> impl Responder {
    let flow = Flow::ThreeDsDecisionRuleExecute;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            three_ds_decision_rule_core::execute_three_ds_decision_rule(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
