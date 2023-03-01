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
    ),
    tag = "Refunds",
    operation_id = "Create a Refund",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsCreate))]
// #[post("")]
pub async fn refunds_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundRequest>,
) -> HttpResponse {
    api::server_wrap(
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        refund_create_core,
        &auth::ApiKeyAuth,
    )
    .await
}

/// Refunds - Retrieve
///
/// To retrieve the properties of a Refund. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/refunds/{refund_id}",
    params(
        ("refund_id" = String, Path, description = "The identifier for refund")
    ),
    responses(
        (status = 200, description = "Refund retrieved", body = RefundResponse),
        (status = 404, description = "Refund does not exist in our records")
    ),
    tag = "Refunds",
    operation_id = "Retrieve a Refund",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsRetrieve))]
// #[get("/{id}")]
pub async fn refunds_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let refund_id = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        refund_id,
        |state, merchant_account, refund_id| {
            refund_response_wrapper(state, merchant_account, refund_id, refund_retrieve_core)
        },
        &auth::ApiKeyAuth,
    )
    .await
}

/// Refunds - Update
///
/// To update the properties of a Refund object. This may include attaching a reason for the refund or metadata fields
#[utoipa::path(
    post,
    path = "/refunds/{refund_id}",
    params(
        ("refund_id" = String, Path, description = "The identifier for refund")
    ),
    request_body=RefundUpdateRequest,
    responses(
        (status = 200, description = "Refund updated", body = RefundResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Refunds",
    operation_id = "Update a Refund",
    security(("api_key" = []))
)]
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
        state.get_ref(),
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| {
            refund_update_core(&*state.store, merchant_account, &refund_id, req)
        },
        &auth::ApiKeyAuth,
    )
    .await
}

/// Refunds - List
///
/// To list the refunds associated with a payment_id or with the merchant, if payment_id is not provided
#[utoipa::path(
    get,
    path = "/refunds/list",
    params(
        ("payment_id" = String, Query, description = "The identifier for the payment"),
        ("limit" = i64, Query, description = "Limit on the number of objects to return"),
        ("created" = PrimitiveDateTime, Query, description = "The time at which refund is created"),
        ("created_lt" = PrimitiveDateTime, Query, description = "Time less than the refund created time"),
        ("created_gt" = PrimitiveDateTime, Query, description = "Time greater than the refund created time"),
        ("created_lte" = PrimitiveDateTime, Query, description = "Time less than or equals to the refund created time"),
        ("created_gte" = PrimitiveDateTime, Query, description = "Time greater than or equals to the refund created time")
    ),
    responses(
        (status = 200, description = "List of refunds", body = RefundListResponse),
        (status = 404, description = "Refund does not exist in our records")
    ),
    tag = "Refunds",
    operation_id = "List all Refunds",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsList))]
#[cfg(feature = "olap")]
// #[get("/list")]
pub async fn refunds_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<api_models::refunds::RefundListRequest>,
) -> HttpResponse {
    api::server_wrap(
        state.get_ref(),
        &req,
        payload.into_inner(),
        |state, merchant_account, req| refund_list(&*state.store, merchant_account, req),
        &auth::ApiKeyAuth,
    )
    .await
}
