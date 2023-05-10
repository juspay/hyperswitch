use actix_web::web;
use router_env::{instrument, tracing};

use self::types::DummyConnectorPaymentRequest;
use super::app;
use crate::services::{api, authentication as auth};

mod errors;
mod types;
mod utils;

#[instrument(skip_all, fields(flow = ?types::Flow::DummyPaymentCreate))]
pub async fn dummy_connector_payment(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<DummyConnectorPaymentRequest>,
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
