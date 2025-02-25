use actix_web::{web, HttpRequest, HttpResponse};
use api_models::recon as recon_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, recon},
    services::{api, authentication, authorization::permissions::Permission},
};

pub async fn update_merchant(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::MerchantId>,
    json_payload: web::Json<recon_api::ReconUpdateMerchantRequest>,
) -> HttpResponse {
    let flow = Flow::ReconMerchantUpdate;
    let merchant_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req, _| recon::recon_merchant_account_update(state, auth, req),
        &authentication::AdminApiAuthWithMerchantIdFromRoute(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn request_for_recon(state: web::Data<AppState>, http_req: HttpRequest) -> HttpResponse {
    let flow = Flow::ReconServiceRequest;
    Box::pin(api::server_wrap(
        flow,
        state,
        &http_req,
        (),
        |state, user, _, _| recon::send_recon_request(state, user),
        &authentication::JWTAuth {
            permission: Permission::MerchantAccountWrite,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn get_recon_token(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::ReconTokenRequest;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user, _, _| recon::generate_recon_token(state, user),
        &authentication::JWTAuth {
            permission: Permission::MerchantReconTokenRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "recon")]
pub async fn verify_recon_token(state: web::Data<AppState>, http_req: HttpRequest) -> HttpResponse {
    let flow = Flow::ReconVerifyToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        (),
        |state, user, _req, _| recon::verify_recon_token(state, user),
        &authentication::JWTAuth {
            permission: Permission::MerchantReconTokenRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
