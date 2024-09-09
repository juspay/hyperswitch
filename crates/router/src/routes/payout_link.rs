use actix_web::{web, Responder};
use api_models::payouts::PayoutLinkInitiateRequest;
use common_utils::consts::DEFAULT_LOCALE;
use router_env::Flow;

use crate::{
    core::{api_locking, payout_link::*},
    headers::ACCEPT_LANGUAGE,
    services::{
        api,
        authentication::{self as auth, get_header_value_by_key},
    },
    AppState,
};
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
    let locale = get_header_value_by_key(ACCEPT_LANGUAGE.into(), headers)
        .ok()
        .flatten()
        .map(|val| val.to_string())
        .unwrap_or(DEFAULT_LOCALE.to_string());
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth, req, _| {
            initiate_payout_link(
                state,
                auth.merchant_account,
                auth.key_store,
                req,
                headers,
                locale.clone(),
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
