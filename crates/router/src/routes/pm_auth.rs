use actix_web::{web, HttpRequest, Responder};
use api_models as api_types;
use router_env::{instrument, tracing, types::Flow};

use crate::{
    core::api_locking,
    routes::AppState,
    services::{api, authentication as auth},
    types::transformers::ForeignTryFrom,
};

#[instrument(skip_all, fields(flow = ?Flow::PmAuthLinkTokenCreate))]
pub async fn link_token_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_types::pm_auth::LinkTokenCreateRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();
    let flow = Flow::PmAuthLinkTokenCreate;
    let api_auth = auth::ApiKeyAuth::default();

    let (auth, _) = match crate::services::authentication::check_client_secret_and_get_auth(
        req.headers(),
        &payload,
        api_auth,
    ) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    let header_payload =
        match hyperswitch_domain_models::payments::HeaderPayload::foreign_try_from(req.headers()) {
            Ok(headers) => headers,
            Err(err) => {
                return api::log_and_return_error_response(err);
            }
        };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, payload, _| {
            let platform = auth.into();
            crate::core::pm_auth::create_link_token(
                state,
                platform,
                payload,
                Some(header_payload.clone()),
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
    let api_auth = auth::ApiKeyAuth::default();

    let (auth, _) = match crate::services::authentication::check_client_secret_and_get_auth(
        req.headers(),
        &payload,
        api_auth,
    ) {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, payload, _| {
            let platform = auth.into();
            crate::core::pm_auth::exchange_token_core(state, platform, payload)
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
