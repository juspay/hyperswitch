use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::{api_locking, payments::helpers},
    services::{api, authentication as auth},
    types::api::customers,
};

#[instrument(skip_all, fields(flow = ?Flow::EphemeralKeyCreate))]
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
