use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::AppState;
use crate::{
    core::payments::helpers,
    services::{api, authentication as auth},
    types::api::customers,
};

#[instrument(skip_all, fields(flow = ?Flow::EphemeralKeyCreate))]
pub async fn ephemeral_key_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerId>,
) -> HttpResponse {
    let payload = json_payload.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, req| {
            helpers::make_ephemeral_key(state, req.customer_id, merchant_account.merchant_id)
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::EphemeralKeyDelete))]
pub async fn ephemeral_key_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = path.into_inner();
    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, _, req| helpers::delete_ephemeral_key(&*state.store, req),
        &auth::ApiKeyAuth,
    )
    .await
}
