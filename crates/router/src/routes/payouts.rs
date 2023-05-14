use actix_web::{
    body::{BoxBody, MessageBody},
    web, HttpRequest, HttpResponse, Responder,
};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::payouts::*,
    services::{api, authentication as auth},
    types::api::payouts as payout_types,
};

/// Payouts - Create
#[utoipa::path(
    post,
    path = "/payouts/create",
    request_body=PayoutCreateRequest,
    responses(
        (status = 200, description = "Payout created", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Create a Payout",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsCreate))]
pub async fn payouts_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsCreate;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        payouts_create_core,
        &auth::ApiKeyAuth,
    )
    .await
}

/// Payouts - Retrieve
#[utoipa::path(
    get,
    path = "/payouts/retrieve/{payout_id}",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout]")
    ),
    responses(
        (status = 200, description = "Payout retrieved", body = PayoutCreateResponse),
        (status = 404, description = "Payout does not exist in our records")
    ),
    tag = "Payouts",
    operation_id = "Retrieve a Payout",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsRetrieve))]
pub async fn payouts_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query_params: web::Query<payout_types::PayoutRetrieveBody>,
) -> HttpResponse {
    let payout_retrieve_request = payout_types::PayoutRetrieveRequest {
        payout_id: path.into_inner(),
        force_sync: query_params.force_sync,
    };
    let flow = Flow::PayoutsRetrieve;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payout_retrieve_request,
        payouts_retrieve_core,
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsUpdate))]
// #[post("/update")]
pub async fn payouts_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsUpdate;
    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        payouts_update_core,
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsCancel))]
// #[post("/cancel")]
pub async fn payouts_cancel() -> impl Responder {
    let _flow = Flow::PayoutsCancel;
    http_response("cancel")
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsFulfill))]
// #[post("/cancel")]
pub async fn payouts_fulfill() -> impl Responder {
    let _flow = Flow::PayoutsFulfill;
    http_response("fulfill")
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
