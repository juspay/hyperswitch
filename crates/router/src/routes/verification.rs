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
) -> impl Responder {
    let flow = Flow::Verification;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload,
        |state, _, body| {
            verification::verify_merchant_creds_for_applepay(state, &req, body, &state.conf.kms)
        },
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}
