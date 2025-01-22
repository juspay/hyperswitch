use actix_web::{web, HttpRequest, HttpResponse};
use api_models::hypersense as hypersense_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, hypersense},
    services::{api, authentication},
};

pub async fn get_hypersense_token(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::HypersenseTokenRequest;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user, _, _| hypersense::generate_hypersense_token(state, user),
        &authentication::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn verify_hypersense_token(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<hypersense_api::HypersenseVerifyTokenRequest>,
) -> HttpResponse {
    let flow = Flow::HypersenseVerifyToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _: (), json_payload, _| hypersense::verify_hypersense_token(state, json_payload),
        &authentication::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
