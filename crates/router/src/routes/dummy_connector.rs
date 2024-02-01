use actix_web::web;
use router_env::{instrument, tracing};

use super::app;
use crate::{
    core::api_locking,
    services::{api, authentication as auth},
};

mod consts;
mod core;
mod errors;
pub mod types;
mod utils;

#[instrument(skip_all, fields(flow = ?types::Flow::DummyPaymentCreate))]
/// This method is used to authorize a dummy payment using the dummy connector. It takes the application state, the HTTP request, and the payment attempt ID as input parameters. It then constructs a payload for the dummy connector payment confirm request and uses the `api::server_wrap` function to wrap the authorization flow, state, request, payload, authorization function, authentication method, and locking action. Finally, it awaits the result and returns the responder.
pub async fn dummy_connector_authorize_payment(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> impl actix_web::Responder {
    let flow = types::Flow::DummyPaymentAuthorize;
    let attempt_id = path.into_inner();
    let payload = types::DummyConnectorPaymentConfirmRequest { attempt_id };
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| core::payment_authorize(state, req),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?types::Flow::DummyPaymentCreate))]
/// This method is used to complete a payment using the dummy connector. It takes in the application state, an HttpRequest, a Path containing the attempt ID, and the JSON payload containing the confirmation details. It then constructs a DummyConnectorPaymentCompleteRequest and uses api::server_wrap to handle the payment completion flow, passing in the necessary parameters. Finally, it awaits the result and returns the response as an actix_web::Responder.
pub async fn dummy_connector_complete_payment(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    json_payload: web::Query<types::DummyConnectorPaymentCompleteBody>,
) -> impl actix_web::Responder {
    let flow = types::Flow::DummyPaymentComplete;
    let attempt_id = path.into_inner();
    let payload = types::DummyConnectorPaymentCompleteRequest {
        attempt_id,
        confirm: json_payload.confirm,
    };
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| core::payment_complete(state, req),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?types::Flow::DummyPaymentCreate))]
/// This method handles a dummy connector payment by extracting the JSON payload and passing it to the core payment function. It wraps the payment flow in a server_wrap function, passing the necessary parameters and awaits for the response.
pub async fn dummy_connector_payment(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<types::DummyConnectorPaymentRequest>,
) -> impl actix_web::Responder {
    let payload = json_payload.into_inner();
    let flow = types::Flow::DummyPaymentCreate;
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| core::payment(state, req),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?types::Flow::DummyPaymentRetrieve))]
/// This method is used to retrieve dummy payment data from the connector. It takes the application state, the HTTP request, and the payment ID as input parameters. It then constructs a payload for the dummy payment retrieval request and passes it to the server_wrap function along with the flow, state, request, and other parameters. The server_wrap function handles the API request, authentication, and locking, and awaits the result before returning it as a responder.
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
        state,
        &req,
        payload,
        |state, _, req| core::payment_data(state, req),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?types::Flow::DummyRefundCreate))]
/// This method handles the refund process for a dummy payment connector. It takes in the app state,
/// the HTTP request, the JSON payload containing the refund request details, and the payment ID
/// from the URL path. It then creates a refund flow, updates the payload with the payment ID, and
/// calls the `server_wrap` function to handle the refund process. Finally, it awaits the result of
/// the `server_wrap` function and returns the response as an actix_web Responder.
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
        state,
        &req,
        payload,
        |state, _, req| core::refund_payment(state, req),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?types::Flow::DummyRefundRetrieve))]
/// This method handles the retrieval of refund data from a dummy connector. It takes the application state, the HTTP request, and the refund ID as input parameters. It returns the retrieved refund data as a responder.
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
        state,
        &req,
        payload,
        |state, _, req| core::refund_data(state, req),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
