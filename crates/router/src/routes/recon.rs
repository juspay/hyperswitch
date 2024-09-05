use actix_web::{web, HttpRequest, HttpResponse};
use api_models::recon as recon_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, recon},
    services::{
        api,
        authentication::{self as auth, UserFromToken},
    },
};

pub async fn update_merchant(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<recon_api::ReconUpdateMerchantRequest>,
) -> HttpResponse {
    let flow = Flow::ReconMerchantUpdate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, _user, req, _| recon::recon_merchant_account_update(state, req),
        &auth::ReconAdmin,
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
        |state, user: UserFromToken, _req, _| recon::send_recon_request(state, user),
        &auth::DashboardNoPermissionAuth,
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
        |state, user: UserFromToken, _, _| recon::generate_recon_token(state, user),
        &auth::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
