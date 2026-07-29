use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app;
use crate::{
    core::{account_updater, api_locking},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::AccountUpdaterConnectivityCheck))]
pub async fn account_updater_connectivity_check(
    state: web::Data<app::AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::AccountUpdaterConnectivityCheck;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _, _, _| account_updater::connectivity::check_account_updater_connectivity(state),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
