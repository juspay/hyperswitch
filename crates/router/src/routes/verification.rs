use actix_web::{web, HttpRequest, Responder};
use api_models::verifications;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    services::{api, authentication as auth},
    utils::verification,
};

#[cfg(all(feature = "olap", feature = "kms"))]
#[instrument(skip_all, fields(flow = ?Flow::Verification))]
pub async fn apple_pay_merchant_registration(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<verifications::ApplepayMerchantVerificationRequest>,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::Verification;
    let merchant_id = path.into_inner();
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload,
        |state, _, body| {
            verification::verify_merchant_creds_for_applepay(state, &req, body, &state.conf.kms, merchant_id.clone())
        },
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}

