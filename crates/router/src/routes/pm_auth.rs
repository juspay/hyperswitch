use actix_web::{web, HttpRequest, Responder};
use api_models as api_types;
use router_env::{instrument, tracing, types::Flow};

use crate::{core::api_locking, routes::AppState, services::api as oss_api};

#[instrument(skip_all, fields(flow = ?Flow::PmAuthLinkTokenCreate))]
pub async fn link_token_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_types::pm_auth::LinkTokenCreateRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();
    let flow = Flow::PmAuthLinkTokenCreate;
    let (auth, _) = match crate::services::authentication::check_client_secret_and_get_auth(
        req.headers(),
        &payload,
    ) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return oss_api::log_and_return_error_response(e),
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, payload| {
            crate::core::pm_auth::create_link_token(
                state,
                auth.merchant_account,
                auth.key_store,
                payload,
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PmAuthExchangeToken))]
pub async fn exchange_token(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_types::pm_auth::ExchangeTokenCreateRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();
    let flow = Flow::PmAuthExchangeToken;
    let (auth, _) = match crate::services::authentication::check_client_secret_and_get_auth(
        req.headers(),
        &payload,
    ) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return oss_api::log_and_return_error_response(e),
    };
    Box::pin(oss_api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, payload| {
            crate::core::pm_auth::exchange_token_core(
                state,
                auth.merchant_account,
                auth.key_store,
                payload,
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
