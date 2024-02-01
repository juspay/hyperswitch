use actix_web::{
    body::{BoxBody, MessageBody},
    web, HttpRequest, HttpResponse, Responder,
};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::api_locking,
    services::{api, authentication as auth},
};
#[cfg(feature = "payouts")]
use crate::{core::payouts::*, types::api::payouts as payout_types};

/// Payouts - Create
#[cfg(feature = "payouts")]
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
/// Asynchronously handles the creation of payouts. 
pub async fn payouts_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req| payouts_create_core(state, auth.merchant_account, auth.key_store, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Retrieve
#[cfg(feature = "payouts")]
#[utoipa::path(
    get,
    path = "/payouts/{payout_id}",
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
/// Retrieves a payout using the provided payout ID and query parameters. This method uses the provided AppState, HttpRequest, path, and query parameters to construct a PayoutRetrieveRequest object and initiates the process of retrieving the payout. It then utilizes the api::server_wrap function to handle the retrieval process and returns the HttpResponse.
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
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payout_retrieve_request,
        |state, auth, req| payouts_retrieve_core(state, auth.merchant_account, auth.key_store, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Update
#[cfg(feature = "payouts")]
#[utoipa::path(
    post,
    path = "/payouts/{payout_id}",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout]")
    ),
    request_body=PayoutCreateRequest,
    responses(
        (status = 200, description = "Payout updated", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Update a Payout",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsUpdate))]
/// This method is used to update a specific payout by its ID. It takes in the current application state, the HTTP request, the requested path, and the JSON payload containing the payout update information. It then constructs the necessary payload, wraps the request in a server context using the specified flow, and calls the `payouts_update_core` method to handle the actual payout update logic. The method returns a response as an HTTP response.
pub async fn payouts_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsUpdate;
    let payout_id = path.into_inner();
    let mut payout_update_payload = json_payload.into_inner();
    payout_update_payload.payout_id = Some(payout_id);
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payout_update_payload,
        |state, auth, req| payouts_update_core(state, auth.merchant_account, auth.key_store, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Cancel
#[cfg(feature = "payouts")]
#[utoipa::path(
    post,
    path = "/payouts/{payout_id}/cancel",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout")
    ),
    request_body=PayoutActionRequest,
    responses(
        (status = 200, description = "Payout cancelled", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Cancel a Payout",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsCancel))]
/// This method handles the cancellation of a payout by sending a request to the server with the provided payload and path parameter. It then wraps the request in a box and awaits the response from the server.
pub async fn payouts_cancel(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutActionRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PayoutsCancel;
    let mut payload = json_payload.into_inner();
    payload.payout_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| payouts_cancel_core(state, auth.merchant_account, auth.key_store, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Fulfill
#[cfg(feature = "payouts")]
#[utoipa::path(
    post,
    path = "/payouts/{payout_id}/fulfill",
    params(
        ("payout_id" = String, Path, description = "The identifier for payout")
    ),
    request_body=PayoutActionRequest,
    responses(
        (status = 200, description = "Payout fulfilled", body = PayoutCreateResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payouts",
    operation_id = "Fulfill a Payout",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFulfill))]
/// Asynchronously fulfills a payout action request by updating the state with the provided data and calling the payouts_fulfill_core method. 
pub async fn payouts_fulfill(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutActionRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PayoutsFulfill;
    let mut payload = json_payload.into_inner();
    payload.payout_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| payouts_fulfill_core(state, auth.merchant_account, auth.key_store, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PayoutsAccounts))]
// #[get("/accounts")]
pub async fn payouts_accounts() -> impl Responder {
    let _flow = Flow::PayoutsAccounts;
    http_response("accounts")
}

/// Takes a response of type T that implements the MessageBody trait and returns an HttpResponse
/// with a BoxBody, which is suitable for streaming large response bodies.
fn http_response<T: MessageBody + 'static>(response: T) -> HttpResponse<BoxBody> {
    HttpResponse::Ok().body(response)
}
