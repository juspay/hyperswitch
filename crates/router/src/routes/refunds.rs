use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{core::refunds::*, services::api, types::api::refunds};

#[instrument(skip_all, fields(flow = ?Flow::RefundsCreate))]
// #[post("")]
pub async fn refunds_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundRequest>,
) -> HttpResponse {
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        refund_create_core,
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RefundsRetrieve))]
// #[get("/{id}")]
pub async fn refunds_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let refund_id = path.into_inner();

    api::server_wrap(
        &state,
        &req,
        refund_id,
        |state, merchant_account, refund_id| {
            refund_response_wrapper(state, merchant_account, refund_id, refund_retrieve_core)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RefundsUpdate))]
// #[post("/{id}")]
pub async fn refunds_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let refund_id = path.into_inner();
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| {
            refund_update_core(&*state.store, merchant_account, &refund_id, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RefundsList))]
// #[get("/list")]
pub async fn refunds_list() -> HttpResponse {
    api::http_response_json("list")
}
