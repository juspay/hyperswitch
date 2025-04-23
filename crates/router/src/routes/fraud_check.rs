use actix_web::{web, HttpRequest, HttpResponse};
use router_env::Flow;

use crate::{
    core::{api_locking, fraud_check as frm_core},
    services::{self, api},
    types::domain,
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
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            frm_core::frm_fulfillment_core(state, merchant_context, req)
        },
        &services::authentication::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
