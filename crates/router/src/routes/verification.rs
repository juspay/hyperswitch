use actix_web::{web, HttpRequest, Responder};
use api_models::verifications;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::verification,
    services::{api, authentication as auth},
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
            verification::verify_merchant_creds_for_applepay(
                state,
                &req,
                body,
                kms_conf,
                merchant_id.clone(),
            )
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
    let merchant_id = &params.merchant_id;
    let mca_id = &params.merchant_connector_account_id;

    api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, _, _| {
            verification::get_verified_apple_domains_with_mid_mca_id(
                &*state.store,
                merchant_id.to_string(),
                mca_id.clone(),
            )
        },
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
    )
    .await
}
