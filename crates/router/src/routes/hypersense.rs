use actix_web::{web, HttpRequest, HttpResponse};
use api_models::{
    external_service_auth as external_service_auth_api,
    external_service_hypersense as external_service_hypersense_api,
};

use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, external_service_auth, external_service_hypersense},
    services::{
        api,
        authentication::{self, ExternalServiceType},
        authorization::permissions::Permission,
    },
};

use router_env::logger;

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

pub async fn get_fee_estimate(
    path: web::Path<String>,
    state: web::Data<AppState>,
    http_req: HttpRequest,
    payload: web::Json<external_service_hypersense_api::ExternalFeeEstimateRequest>,
) -> HttpResponse {
    let payload_inner = payload.into_inner();
    let query_params = http_req.query_string();

    logger::info!(
        "Received fee estimate request for path: {}, query params: {:?} and payload: {:?}",
        path,
        query_params,
        serde_json::to_string(&payload_inner)
            .unwrap_or_else(|_| "Failed to serialize payload".to_string())
    );

    let flow = Flow::HypersenseFeeEstimate;
    let api_path = path.into_inner();
    let json_payload = web::Json(
        external_service_hypersense_api::ExternalFeeEstimatePayload {
            payload: payload_inner.payload,
        },
    );

    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, user: authentication::UserFromToken, json_payload, _| {
            external_service_hypersense::get_hypersense_fee_estimate(
                state,
                api_path.clone(),
                query_params,
                json_payload,
                user.clone()
            )
        },
        &authentication::JWTAuth {
            permission: Permission::MerchantAnalyticsRead,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
