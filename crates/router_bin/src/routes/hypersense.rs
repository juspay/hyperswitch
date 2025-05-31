use actix_web::{web, HttpRequest, HttpResponse};
use api_models::external_service_auth as external_service_auth_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, external_service_auth},
    services::{
        api,
        authentication::{self, ExternalServiceType},
    },
};

pub async fn get_hypersense_token(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::HypersenseTokenRequest;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, user, _, _| {
            external_service_auth::generate_external_token(
                state,
                user,
                ExternalServiceType::Hypersense,
            )
        },
        &authentication::DashboardNoPermissionAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn signout_hypersense_token(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<external_service_auth_api::ExternalSignoutTokenRequest>,
) -> HttpResponse {
    let flow = Flow::HypersenseSignoutToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _: (), json_payload, _| {
            external_service_auth::signout_external_token(state, json_payload)
        },
        &authentication::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn verify_hypersense_token(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<external_service_auth_api::ExternalVerifyTokenRequest>,
) -> HttpResponse {
    let flow = Flow::HypersenseVerifyToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _: (), json_payload, _| {
            external_service_auth::verify_external_token(
                state,
                json_payload,
                ExternalServiceType::Hypersense,
            )
        },
        &authentication::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
