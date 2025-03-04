use actix_web::{web, HttpRequest, Responder};
use api_models::verifications;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, verification},
    services::{api, authentication as auth, authorization::permissions::Permission},
};

#[cfg(all(feature = "olap", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::Verification))]
pub async fn apple_pay_merchant_registration(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<verifications::ApplepayMerchantVerificationRequest>,
    path: web::Path<common_utils::id_type::MerchantId>,
) -> impl Responder {
    let flow = Flow::Verification;
    let merchant_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body, _| {
            verification::verify_merchant_creds_for_applepay(
                state.clone(),
                body,
                merchant_id.clone(),
                auth.profile_id,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileAccountWrite,
            },
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

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        merchant_id.clone(),
        |state, _: auth::AuthenticationData, _, _| {
            verification::get_verified_apple_domains_with_mid_mca_id(
                state,
                merchant_id.to_owned(),
                mca_id.clone(),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantAccountRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
