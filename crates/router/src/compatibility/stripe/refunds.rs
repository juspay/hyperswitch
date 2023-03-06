pub mod types;

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing};

use crate::{
    compatibility::{stripe::errors, wrap},
    core::refunds,
    routes,
    services::authentication as auth,
    types::api::refunds as refund_types,
};

#[instrument(skip_all)]
#[post("")]
pub async fn refund_create(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    form_payload: web::Form<types::StripeCreateRefundRequest>,
) -> HttpResponse {
    let payload = form_payload.into_inner();
    let create_refund_req: refund_types::RefundRequest = payload.into();

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeCreateRefundResponse,
        errors::StripeErrorCode,
    >(
        state.get_ref(),
        &req,
        create_refund_req,
        refunds::refund_create_core,
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all)]
#[get("/{refund_id}")]
pub async fn refund_retrieve(
    state: web::Data<routes::AppState>,
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
        _,
        types::StripeCreateRefundResponse,
        errors::StripeErrorCode,
    >(
        state.get_ref(),
        &req,
        refund_id,
        |state, merchant_account, refund_id| {
            refunds::refund_response_wrapper(
                state,
                merchant_account,
                refund_id,
                refunds::refund_retrieve_core,
            )
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{refund_id}")]
pub async fn refund_update(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    form_payload: web::Form<types::StripeUpdateRefundRequest>,
) -> HttpResponse {
    let refund_id = path.into_inner();
    let payload = form_payload.into_inner();
    let create_refund_update_req: refund_types::RefundUpdateRequest = payload.into();

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeCreateRefundResponse,
        errors::StripeErrorCode,
    >(
        state.get_ref(),
        &req,
        create_refund_update_req,
        |state, merchant_account, req| {
            refunds::refund_update_core(&*state.store, merchant_account, &refund_id, req)
        },
        &auth::ApiKeyAuth,
    )
    .await
}
