use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::refunds::*,
    services::{api, authentication as auth},
    types::api::refunds,
};

/// Refunds - Create
///
/// To create a refund against an already processed payment
#[utoipa::path(
    post,
    path = "/refunds",
    request_body=RefundRequest,
    responses(
        (status = 200, description = "Refund created", body = RefundResponse),
        (status = 400, description = "Missing Mandatory fields")
    )
)]
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
        &auth::ApiKeyAuth,
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
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RefundsUpdate))]
// #[post("/{id}")]
pub async fn refunds_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundUpdateRequest>,
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
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RefundsList))]
#[cfg(feature = "olap")]
// #[get("/list")]
pub async fn refunds_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<api_models::refunds::RefundListRequest>,
) -> HttpResponse {
    api::server_wrap(
        &state,
        &req,
        payload.into_inner(),
        |state, merchant_account, req| refund_list(&*state.store, merchant_account, req),
        &auth::ApiKeyAuth,
    )
    .await
}
