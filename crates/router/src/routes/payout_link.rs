use actix_web::{web, Responder};
use router_env::Flow;

use crate::{
    core::{api_locking, payout_link::*},
    services::{api, authentication as auth},
    AppState,
};
pub async fn render_payout_link(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::PayoutLinkInitiate;
    let (merchant_id, payout_id) = path.into_inner();
    let payload = api_models::payouts::PayoutLinkInitiateRequest {
        merchant_id: merchant_id.clone(),
        payout_id,
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth, req, _| {
            initiate_payout_link(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
