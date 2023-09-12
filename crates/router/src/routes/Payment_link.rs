use actix_web::{web, Responder};
use router_env::Flow;

use crate::{
    self as app,
    core::payments::{self},
    services::{api, authentication as auth},
};

pub async fn get_payment_link(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    query_payload: web::Query<api_models::payments::RetrievePaymentLinkRequest>,
) -> impl Responder {
    let flow = Flow::PaymentLinkRetrieve;

    let payload = query_payload.into_inner();

    let (auth_type, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
    };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload.clone(),
        |state, auth, _| {
            payments::retrieve_payment_link(
                state,
                auth.merchant_account,
                payload.payment_link_id.clone(),
            )
        },
        &*auth_type,
    )
    .await
}


pub async fn initiate_payment_link (
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::PaymentLinkInitiate;
    let (merchant_id, payment_id) = path.into_inner();

    let payload = web::Json(api_models::payments::PaymentLinkInitiateRequest {
        payment_id,
        merchant_id : merchant_id.clone() ,
    }).into_inner();

    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload.clone(),
        |state, auth , _| {
            payments::intiate_payment_link(
                state,
                auth.merchant_account,
                payload.merchant_id.clone(),
                payload.payment_id.clone(),
            )
        },
        &auth::MerchantIdAuth(merchant_id),
    )
    .await
}