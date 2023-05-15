use actix_web::web;
use router_env::{instrument, tracing};

use super::app;
use crate::services::{api, authentication as auth};

mod errors;
mod types;
mod utils;

#[instrument(skip_all, fields(flow = ?types::Flow::DummyPaymentCreate))]
pub async fn dummy_connector_payment(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<types::DummyConnectorPaymentRequest>,
) -> impl actix_web::Responder {
    let flow = types::Flow::DummyPaymentCreate;
    let payload = json_payload.into_inner();
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, _, req| utils::payment(state, req),
        &auth::NoAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?types::Flow::DummyPaymentRetrieve))]
pub async fn dummy_connector_payment_data(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let flow = types::Flow::DummyPaymentRetrieve;
    let payment_id = path.into_inner();
    let payload = types::DummyConnectorPaymentRetrieveRequest { payment_id };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, _, req| utils::payment_data(state, req),
        &auth::NoAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?types::Flow::DummyRefundCreate))]
pub async fn dummy_connector_refund(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<types::DummyConnectorRefundRequest>,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let flow = types::Flow::DummyRefundCreate;
    let mut payload = json_payload.into_inner();
    payload.payment_id = Some(path.to_string());
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, _, req| utils::refund_payment(state, req),
        &auth::NoAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?types::Flow::DummyRefundRetrieve))]
pub async fn dummy_connector_refund_data(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let flow = types::Flow::DummyRefundRetrieve;
    let refund_id = path.into_inner();
    let payload = types::DummyConnectorRefundRetrieveRequest { refund_id };
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, _, req| utils::refund_data(state, req),
        &auth::NoAuth,
    )
    .await
}
