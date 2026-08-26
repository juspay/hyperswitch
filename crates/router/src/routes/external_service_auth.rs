use actix_web::{web, HttpRequest, HttpResponse};
use api_models::external_service_auth as external_service_auth_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, external_service_auth},
    services::{api, authentication},
};

pub async fn validate_token(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<external_service_auth_api::ValidateTokenRequest>,
) -> HttpResponse {
    let flow = Flow::ExternalServiceValidateToken;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &http_req,
        json_payload.into_inner(),
        |state, _: (), payload, _| external_service_auth::validate_token(state, payload),
        &authentication::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
