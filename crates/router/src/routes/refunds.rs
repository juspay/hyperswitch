use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, refunds::*},
    services::{api, authentication as auth, authorization::permissions::Permission},
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
    let flow = Flow::RefundsCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req, _| refund_create_core(state, auth.merchant_account, auth.key_store, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RefundWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Refunds - Retrieve (GET)
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
#[instrument(skip_all, fields(flow))]
// #[get("/{id}")]
pub async fn refunds_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query_params: web::Query<api_models::refunds::RefundsRetrieveBody>,
) -> HttpResponse {
    let refund_request = refunds::RefundsRetrieveRequest {
        refund_id: path.into_inner(),
        force_sync: query_params.force_sync,
        merchant_connector_details: None,
    };
    let flow = match query_params.force_sync {
        Some(true) => Flow::RefundsRetrieveForceSync,
        _ => Flow::RefundsRetrieve,
    };

    tracing::Span::current().record("flow", &flow.to_string());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        refund_request,
        |state, auth, refund_request, _| {
            refund_response_wrapper(
                state,
                auth.merchant_account,
                auth.key_store,
                refund_request,
                refund_retrieve_core,
            )
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RefundRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Refunds - Retrieve (POST)
///
/// To retrieve the properties of a Refund. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/refunds/sync",
    responses(
        (status = 200, description = "Refund retrieved", body = RefundResponse),
        (status = 404, description = "Refund does not exist in our records")
    ),
    tag = "Refunds",
    operation_id = "Retrieve a Refund",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow))]
// #[post("/sync")]
pub async fn refunds_retrieve_with_body(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundsRetrieveRequest>,
) -> HttpResponse {
    let flow = match json_payload.force_sync {
        Some(true) => Flow::RefundsRetrieveForceSync,
        _ => Flow::RefundsRetrieve,
    };

    tracing::Span::current().record("flow", &flow.to_string());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req, _| {
            refund_response_wrapper(
                state,
                auth.merchant_account,
                auth.key_store,
                req,
                refund_retrieve_core,
            )
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
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
    let flow = Flow::RefundsUpdate;
    let mut refund_update_req = json_payload.into_inner();
    refund_update_req.refund_id = path.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        refund_update_req,
        |state, auth, req, _| refund_update_core(state, auth.merchant_account, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Refunds - List
///
/// To list the refunds associated with a payment_id or with the merchant, if payment_id is not provided
#[utoipa::path(
    post,
    path = "/refunds/list",
    request_body=RefundListRequest,
    responses(
        (status = 200, description = "List of refunds", body = RefundListResponse),
    ),
    tag = "Refunds",
    operation_id = "List all Refunds",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsList))]
#[cfg(feature = "olap")]
pub async fn refunds_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<api_models::refunds::RefundListRequest>,
) -> HttpResponse {
    let flow = Flow::RefundsList;
    api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth, req, _| refund_list(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RefundRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Refunds - Filter
///
/// To list the refunds filters associated with list of connectors, currencies and payment statuses
#[utoipa::path(
    post,
    path = "/refunds/filter",
    request_body=TimeRange,
    responses(
        (status = 200, description = "List of filters", body = RefundListMetaData),
    ),
    tag = "Refunds",
    operation_id = "List all filters for Refunds",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsList))]
#[cfg(feature = "olap")]
pub async fn refunds_filter_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<api_models::payments::TimeRange>,
) -> HttpResponse {
    let flow = Flow::RefundsList;
    api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth, req, _| refund_filter_list(state, auth.merchant_account, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RefundRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Refunds - Filter V2
///
/// To list the refunds filters associated with list of connectors, currencies and payment statuses
#[utoipa::path(
    get,
    path = "/refunds/filter_v2",
    responses(
        (status = 200, description = "List of static filters", body = RefundListFilters),
    ),
    tag = "Refunds",
    operation_id = "List all filters for Refunds",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsFilters))]
#[cfg(feature = "olap")]
pub async fn get_refunds_filters(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::RefundsFilters;
    api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth, _, _| get_filters_for_refunds(state, auth.merchant_account),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::RefundRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
