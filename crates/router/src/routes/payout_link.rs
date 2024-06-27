#[cfg(feature = "payouts")]
use actix_web::{web, Responder};
#[cfg(feature = "payouts")]
use api_models::payouts::PayoutLinkInitiateRequest;
#[cfg(feature = "payouts")]
use router_env::Flow;

#[cfg(feature = "payouts")]
use crate::{
    core::{api_locking, payout_link::*},
    services::{api, authentication as auth},
    AppState,
};
#[cfg(feature = "payouts")]
pub async fn render_payout_link(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::PayoutLinkInitiate;
    let (merchant_id, payout_id) = path.into_inner();
    let payload = PayoutLinkInitiateRequest {
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
