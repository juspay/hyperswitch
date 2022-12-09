use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use super::AppState;
use crate::{core::payments::helpers, services::api, types::api::customers};

#[instrument(skip_all, fields(flow = ?Flow::EphemeralKeyCreate))]
pub async fn ephemeral_key_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerId>,
) -> HttpResponse {
    let payload = json_payload.into_inner();
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            helpers::make_ephemeral_key(state, req.customer_id, merchant_account.merchant_id)
        },
        api::MerchantAuthentication::ApiKey,
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
        &state,
        &req,
        payload,
        |state, _, req| helpers::delete_ephemeral_key(&*state.store, req),
        api::MerchantAuthentication::ApiKey,
    )
    .await
}
