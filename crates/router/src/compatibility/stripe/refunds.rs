mod types;

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use router_env::{tracing, tracing::instrument};

use crate::{
    compatibility::{stripe, wrap},
    core::refunds,
    routes::AppState,
    services::api,
    types::api::refunds::RefundRequest,
};

#[instrument(skip_all)]
#[post("")]
pub(crate) async fn refund_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    form_payload: web::Form<types::StripeCreateRefundRequest>,
) -> HttpResponse {
    let payload = form_payload.into_inner();
    let create_refund_req: RefundRequest = payload.into();

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripeCreateRefundResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        create_refund_req,
        refunds::refund_create_core,
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[get("/{refund_id}")]
pub(crate) async fn refund_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let refund_id = path.into_inner();
    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripeCreateRefundResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        refund_id,
        |state, merchant_account, refund_id| {
            refunds::refund_retrieve_core(state, merchant_account, refund_id)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{refund_id}")]
pub(crate) async fn refund_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    form_payload: web::Form<types::StripeCreateRefundRequest>,
) -> HttpResponse {
    let refund_id = path.into_inner();
    let payload = form_payload.into_inner();
    let create_refund_update_req: RefundRequest = payload.into();

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripeCreateRefundResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        create_refund_update_req,
        |state, merchant_account, req| {
            refunds::refund_update_core(&state.store, merchant_account, &refund_id, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}
