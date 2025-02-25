use actix_web::{web, Responder};
use api_models::payouts::PayoutLinkInitiateRequest;
use router_env::Flow;

use crate::{
    core::{api_locking, payout_link::*},
    services::{
        api,
        authentication::{self as auth},
    },
    AppState,
};
#[cfg(feature = "v1")]
pub async fn render_payout_link(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::PayoutLinkInitiate;
    let (merchant_id, payout_id) = path.into_inner();
    let payload = PayoutLinkInitiateRequest {
        merchant_id: merchant_id.clone(),
        payout_id,
    };
    let headers = req.headers();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth, req, _| {
            initiate_payout_link(state, auth.merchant_account, auth.key_store, req, headers)
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
