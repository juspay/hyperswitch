use actix_web::{web, HttpRequest, HttpResponse};
use common_utils;
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "refunds_v2")))]
use crate::core::refunds::*;
#[cfg(all(feature = "v2", feature = "refunds_v2"))]
use crate::core::refunds_v2::*;
use crate::{
    core::api_locking,
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api::refunds,
};

#[cfg(feature = "v2")]
/// A private module to hold internal types to be used in route handlers.
/// This is because we will need to implement certain traits on these types which will have the resource id
/// But the api payload will not contain the resource id
/// So these types can hold the resource id along with actual api payload, on which api event and locking action traits can be implemented
mod internal_payload_types {
    use super::*;

    // Serialize is implemented because of api events
    #[derive(Debug, serde::Serialize)]
    pub struct RefundsGenericRequestWithResourceId<T: serde::Serialize> {
        pub global_refund_id: common_utils::id_type::GlobalRefundId,
        pub payment_id: Option<common_utils::id_type::GlobalPaymentId>,
        #[serde(flatten)]
        pub payload: T,
    }

    impl<T: serde::Serialize> common_utils::events::ApiEventMetric
        for RefundsGenericRequestWithResourceId<T>
    {
        fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
            let refund_id = self.global_refund_id.clone();
            self.payment_id
                .clone()
                .map(|payment_id| common_utils::events::ApiEventsType::Refund {
                    payment_id,
                    refund_id,
                })
        }
    }
}

/// Refunds - Create
///
/// To create a refund against an already processed payment
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "refunds_v2")))]
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
        |state, auth: auth::AuthenticationData, req, _| {
            refund_create_core(
                state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "refunds_v2"))]
#[instrument(skip_all, fields(flow = ?Flow::RefundsCreate))]
// #[post("")]
pub async fn refunds_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundsCreateRequest>,
) -> HttpResponse {
    let flow = Flow::RefundsCreate;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            refund_create_core(state, auth.merchant_account, auth.key_store, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "refunds_v2")))]
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

    tracing::Span::current().record("flow", flow.to_string());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        refund_request,
        |state, auth: auth::AuthenticationData, refund_request, _| {
            refund_response_wrapper(
                state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                refund_request,
                refund_retrieve_core_with_refund_id,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "refunds_v2")))]
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

    tracing::Span::current().record("flow", flow.to_string());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            refund_response_wrapper(
                state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                req,
                refund_retrieve_core_with_refund_id,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "refunds_v2")))]
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
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        refund_update_req,
        |state, auth: auth::AuthenticationData, req, _| {
            refund_update_core(state, auth.merchant_account, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[cfg(all(feature = "v2", feature = "refunds_v2"))]
/// Refunds - Update
///
/// To update the properties of a Refund object. This may include attaching a reason for the refund or metadata fields or updating merchant reference id.
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
    path: web::Path<common_utils::id_type::GlobalRefundId>,
) -> HttpResponse {
    let flow = Flow::RefundsUpdate;

    let global_refund_id = path.into_inner();
    let internal_payload = internal_payload_types::RefundsGenericRequestWithResourceId {
        global_refund_id: global_refund_id.clone(),
        payment_id: None,
        payload: json_payload.into_inner(),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        internal_payload,
        |state, auth: auth::AuthenticationData, req, _| {
            refund_update_core(
                state,
                auth.merchant_account,
                req.payload,
                global_refund_id.clone(),
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
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
pub async fn refunds_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<api_models::refunds::RefundListRequest>,
) -> HttpResponse {
    let flow = Flow::RefundsList;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            refund_list(state, auth.merchant_account, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
/// Refunds - List at profile level
///
/// To list the refunds associated with a payment_id or with the merchant, if payment_id is not provided
#[utoipa::path(
    post,
    path = "/refunds/profile/list",
    request_body=RefundListRequest,
    responses(
        (status = 200, description = "List of refunds", body = RefundListResponse),
    ),
    tag = "Refunds",
    operation_id = "List all Refunds",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsList))]
pub async fn refunds_list_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<api_models::refunds::RefundListRequest>,
) -> HttpResponse {
    let flow = Flow::RefundsList;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            refund_list(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
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
pub async fn refunds_filter_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<common_utils::types::TimeRange>,
) -> HttpResponse {
    let flow = Flow::RefundsList;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            refund_filter_list(state, auth.merchant_account, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
/// Refunds - Filter V2
///
/// To list the refunds filters associated with list of connectors, currencies and payment statuses
#[utoipa::path(
    get,
    path = "/refunds/v2/filter",
    responses(
        (status = 200, description = "List of static filters", body = RefundListFilters),
    ),
    tag = "Refunds",
    operation_id = "List all filters for Refunds",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsFilters))]
pub async fn get_refunds_filters(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::RefundsFilters;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            get_filters_for_refunds(state, auth.merchant_account, None)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
/// Refunds - Filter V2 at profile level
///
/// To list the refunds filters associated with list of connectors, currencies and payment statuses
#[utoipa::path(
    get,
    path = "/refunds/v2/profile/filter",
    responses(
        (status = 200, description = "List of static filters", body = RefundListFilters),
    ),
    tag = "Refunds",
    operation_id = "List all filters for Refunds",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::RefundsFilters))]
pub async fn get_refunds_filters_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::RefundsFilters;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            get_filters_for_refunds(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
#[instrument(skip_all, fields(flow = ?Flow::RefundsAggregate))]
pub async fn get_refunds_aggregates(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_params: web::Query<common_utils::types::TimeRange>,
) -> HttpResponse {
    let flow = Flow::RefundsAggregate;
    let query_params = query_params.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query_params,
        |state, auth: auth::AuthenticationData, req, _| {
            get_aggregates_for_refunds(state, auth.merchant_account, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
#[instrument(skip_all, fields(flow = ?Flow::RefundsManualUpdate))]
pub async fn refunds_manual_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Json<api_models::refunds::RefundManualUpdateRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::RefundsManualUpdate;
    let mut refund_manual_update_req = payload.into_inner();
    refund_manual_update_req.refund_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        refund_manual_update_req,
        |state, _auth, req, _| refund_manual_update(state, req),
        &auth::AdminApiAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "refunds_v2"),
    feature = "olap"
))]
#[instrument(skip_all, fields(flow = ?Flow::RefundsAggregate))]
pub async fn get_refunds_aggregate_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_params: web::Query<common_utils::types::TimeRange>,
) -> HttpResponse {
    let flow = Flow::RefundsAggregate;
    let query_params = query_params.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query_params,
        |state, auth: auth::AuthenticationData, req, _| {
            get_aggregates_for_refunds(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
