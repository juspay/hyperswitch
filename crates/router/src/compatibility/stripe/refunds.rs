pub mod types;
use actix_web::{web, HttpRequest, HttpResponse};
use error_stack::report;
use router_env::{instrument, tracing, Flow};

use crate::{
    compatibility::{stripe::errors, wrap},
    core::{api_locking, refunds},
    routes,
    services::{api, authentication as auth},
    types::api::refunds as refund_types,
};

#[instrument(skip_all, fields(flow = ?Flow::RefundsCreate))]
pub async fn refund_create(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::StripeCreateRefundRequest = match qs_config
        .deserialize_bytes(&form_payload)
        .map_err(|err| report!(errors::StripeErrorCode::from(err)))
    {
        Ok(p) => p,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let create_refund_req: refund_types::RefundRequest = payload.into();

    let flow = Flow::RefundsCreate;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeRefundResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        create_refund_req,
        |state, auth, req| {
            refunds::refund_create_core(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::RefundsRetrieve))]
pub async fn refund_retrieve_with_gateway_creds(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let refund_request = match qs_config
        .deserialize_bytes(&form_payload)
        .map_err(|err| report!(errors::StripeErrorCode::from(err)))
    {
        Ok(payload) => payload,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let flow = Flow::RefundsRetrieve;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeRefundResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        refund_request,
        |state, auth, refund_request| {
            refunds::refund_response_wrapper(
                state,
                auth.merchant_account,
                auth.key_store,
                refund_request,
                refunds::refund_retrieve_core,
            )
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::RefundsRetrieve))]
pub async fn refund_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let refund_request = refund_types::RefundsRetrieveRequest {
        refund_id: path.into_inner(),
        force_sync: Some(true),
        merchant_connector_details: None,
    };

    let flow = Flow::RefundsRetrieve;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeRefundResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        refund_request,
        |state, auth, refund_request| {
            refunds::refund_response_wrapper(
                state,
                auth.merchant_account,
                auth.key_store,
                refund_request,
                refunds::refund_retrieve_core,
            )
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::RefundsUpdate))]
pub async fn refund_update(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    form_payload: web::Form<types::StripeUpdateRefundRequest>,
) -> HttpResponse {
    let mut payload = form_payload.into_inner();
    payload.refund_id = path.into_inner();
    let create_refund_update_req: refund_types::RefundUpdateRequest = payload.into();
    let flow = Flow::RefundsUpdate;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeRefundResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        create_refund_update_req,
        |state, auth, req| refunds::refund_update_core(state, auth.merchant_account, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
