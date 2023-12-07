use actix_web::{web, HttpRequest, HttpResponse};
use api_models::connector_onboarding as api_types;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, connector_onboarding as core},
    services::{api, authentication as auth, authorization::permissions::Permission},
};

pub async fn get_action_url(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<api_types::ActionUrlRequest>,
) -> HttpResponse {
    let flow = Flow::GetActionUrl;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _: auth::UserFromToken, req| core::get_action_url(state, req),
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn sync_onboarding_status(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<api_types::OnboardingSyncRequest>,
) -> HttpResponse {
    let flow = Flow::SyncOnboardingStatus;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        core::sync_onboarding_status,
        &auth::JWTAuth(Permission::MerchantAccountWrite),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
