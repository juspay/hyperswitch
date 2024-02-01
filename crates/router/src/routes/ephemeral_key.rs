use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::{api_locking, payments::helpers},
    services::{api, authentication as auth},
    types::api::customers,
};

#[instrument(skip_all, fields(flow = ?Flow::EphemeralKeyCreate))]
/// Asynchronously handles the creation of an ephemeral key for a customer. It extracts the necessary data from the request, creates a flow for ephemeral key creation, and uses the `api::server_wrap` function to perform the actual key creation. The method returns an `HttpResponse` with the result of the key creation process.
pub async fn ephemeral_key_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerId>,
) -> HttpResponse {
    let flow = Flow::EphemeralKeyCreate;
    let payload = json_payload.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            helpers::make_ephemeral_key(state, req.customer_id, auth.merchant_account.merchant_id)
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::EphemeralKeyDelete))]
/// Handles the deletion of an ephemeral key by making a request to the server with the provided payload and authentication.
pub async fn ephemeral_key_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::EphemeralKeyDelete;
    let payload = path.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _, req| helpers::delete_ephemeral_key(state, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
