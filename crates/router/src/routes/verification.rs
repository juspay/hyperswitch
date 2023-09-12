use actix_web::{web, HttpRequest, Responder};
#[cfg(all(feature = "olap", feature = "kms"))]
use api_models::verifications;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    services::{api, authentication as auth},
    utils::verification,
};

#[instrument(skip_all, fields(flow = ?Flow::Verification))]
pub async fn apple_pay_merchant_registration(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<verifications::ApplepayMerchantVerificationRequest>,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::Verification;
    let merchant_id = path.into_inner();
    let kms_conf = &state.clone().conf.kms;
    api::server_wrap(
        flow,
        state,
        &req,
        json_payload,
        |state, _, body| {
            verification::verify_merchant_creds_for_applepay(state, &req, body, kms_conf)
        },
        &auth::MerchantIdAuth(merchant_id.clone()),
    )
    .await
}
