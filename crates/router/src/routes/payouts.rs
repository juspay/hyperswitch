use actix_web::{
    body::{BoxBody, MessageBody},
    HttpResponse, Responder,
};
use router_env::{instrument, tracing, Flow};

#[instrument(skip_all, fields(flow = ?Flow::PayoutsCreate))]
// #[post("/create")]
pub async fn payouts_create() -> impl Responder {
    let _flow = Flow::PayoutsCreate;
    http_response("create")
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
