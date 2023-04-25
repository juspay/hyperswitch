use actix_web::web;
use router_env::{instrument, tracing};

use self::types::DummyConnectorPaymentsRequest;
use super::app;
use crate::services::{api, authentication as auth};

mod errors;
mod types;
mod utils;

#[instrument(skip_all, fields(flow = ?types::Flow::PaymentCreate))]
pub async fn dummy_connector_payment(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<DummyConnectorPaymentsRequest>,
) -> impl actix_web::Responder {
    let flow = types::Flow::PaymentCreate;
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
