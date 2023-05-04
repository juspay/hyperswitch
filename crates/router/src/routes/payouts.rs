use actix_web::{
    body::{BoxBody, MessageBody},
    web, HttpRequest, HttpResponse, Responder,
};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::payouts::*,
    services::{api, authentication as auth},
    types::api::payouts,
};

#[instrument(skip_all, fields(flow = ?Flow::PayoutsCreate))]
// #[post("/create")]
pub async fn payouts_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payouts::PayoutCreateRequest>,
) -> impl Responder {
    let flow = Flow::PayoutsCreate;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        payout_create_core,
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsRetrieve))]
// #[get("/retrieve")]
pub async fn payouts_retrieve() -> impl Responder {
    let _flow = Flow::PayoutsRetrieve;
    http_response("retrieve")
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsUpdate))]
// #[post("/update")]
pub async fn payouts_update() -> impl Responder {
    let _flow = Flow::PayoutsUpdate;
    http_response("update")
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsReverse))]
// #[post("/reverse")]
pub async fn payouts_reverse() -> impl Responder {
    let _flow = Flow::PayoutsReverse;
    http_response("reverse")
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsCancel))]
// #[post("/cancel")]
pub async fn payouts_cancel() -> impl Responder {
    let _flow = Flow::PayoutsCancel;
    http_response("cancel")
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsAccounts))]
// #[get("/accounts")]
pub async fn payouts_accounts() -> impl Responder {
    let _flow = Flow::PayoutsAccounts;
    http_response("accounts")
}

fn http_response<T: MessageBody + 'static>(response: T) -> HttpResponse<BoxBody> {
    HttpResponse::Ok().body(response)
}
