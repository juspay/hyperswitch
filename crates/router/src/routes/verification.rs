use actix_web::{web, HttpRequest, Responder};
#[cfg(all(feature = "olap", feature = "kms"))]
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

#[instrument(skip_all, fields(flow = ?Flow::Verification))]
pub async fn retrieve_apple_pay_verified_domains(
    state: web::Data<AppState>,
    req: HttpRequest,
    params: web::Query<verifications::ApplepayGetVerifiedDomainsParam>,
) -> impl Responder {
    let flow = Flow::Verification;
    let business_profile_id = &params.business_profile_id;

    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        business_profile_id.clone(),
        |state, _, _| {
            verification::get_verified_apple_domains_with_business_profile_id(
                &*state.store,
                business_profile_id.clone(),
            )
        },
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}
