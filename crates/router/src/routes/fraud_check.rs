use actix_web::{web, HttpRequest, HttpResponse};
use router_env::Flow;

use crate::{
    core::{api_locking, fraud_check as frm_core},
    services::{self, api},
    AppState,
};

#[cfg(feature = "v1")]
pub async fn frm_fulfillment(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<frm_core::types::FrmFulfillmentRequest>,
) -> HttpResponse {
    let flow = Flow::FrmFulfillment;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        json_payload.into_inner(),
        |state, auth: services::authentication::AuthenticationData, req, _| {
            frm_core::frm_fulfillment_core(state, auth.merchant_account, auth.key_store, req)
        },
        &services::authentication::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
