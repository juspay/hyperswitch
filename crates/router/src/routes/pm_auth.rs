use actix_web::{web, HttpRequest, Responder};
use api_models as api_types;
use router_env::{instrument, tracing, types::Flow};

use crate::{core::api_locking, routes::AppState, services::api as oss_api};

#[instrument(skip_all, fields(flow = ?Flow::PmAuthLinkTokenCreate))]
/// This method is used to create a link token for merchant account authentication. It takes in the Appstate, HttpRequest, and a JSON payload containing the link token create request. It then checks the client secret and gets the authentication details, and finally creates the link token using the provided state, authentication details, and payload. The method returns a Responder as an asynchronous result.
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
/// Handles the exchange token request for merchant account authentication. 
/// It checks the client secret and retrieves the authentication information. 
/// Then it calls the exchange_token_core method from the pm_auth module 
/// to perform the token exchange process. Finally, it wraps the result in 
/// a server response using server_wrap method from oss_api module. 
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
