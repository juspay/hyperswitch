use actix_web::{web, HttpRequest, Responder};
use api_models::verifications;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, verification},
    services::{api, authentication as auth, authorization::permissions::Permission},
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
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _, body| {
            verification::verify_merchant_creds_for_applepay(
                state.clone(),
                body,
                merchant_id.clone(),
            )
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MerchantAccountWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
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
                state,
                merchant_id.to_string(),
                mca_id.to_string(),
            )
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MerchantAccountRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
