use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
#[cfg(feature = "v1")]
use crate::core::refunds::*;
#[cfg(feature = "v2")]
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
            let payment_id = self.payment_id.clone();
            Some(common_utils::events::ApiEventsType::Refund {
                payment_id,
                refund_id,
            })
        }
    }
}

/// Refunds - Create
///
/// To create a refund against an already processed payment
#[cfg(feature = "v1")]
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
            let platform = auth.clone().into();
            refund_create_core(state, platform, auth.profile_id, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundWrite,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::RefundsCreate))]
// #[post("")]
pub async fn refunds_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundsCreateRequest>,
) -> HttpResponse {
    let flow = Flow::RefundsCreate;

    let global_refund_id =
        common_utils::id_type::GlobalRefundId::generate(&state.conf.cell_information.id);
    let payload = json_payload.into_inner();

    let internal_refund_create_payload =
        internal_payload_types::RefundsGenericRequestWithResourceId {
            global_refund_id: global_refund_id.clone(),
            payment_id: Some(payload.payment_id.clone()),
            payload,
        };

    let auth_type = if state.conf.merchant_id_auth.merchant_id_auth_enabled {
        &auth::MerchantIdAuth
    } else {
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            },
            &auth::JWTAuth {
                permission: Permission::ProfileRefundWrite,
            },
            req.headers(),
        )
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        internal_refund_create_payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            refund_create_core(state, platform, req.payload, global_refund_id.clone())
        },
        auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
/// Refunds - Retrieve (GET)
///
/// To retrieve the properties of a Refund. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
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
        all_keys_required: query_params.all_keys_required,
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
            let platform = auth.clone().into();
            refund_response_wrapper(
                state,
                platform,
                auth.profile_id,
                refund_request,
                refund_retrieve_core_with_refund_id,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow))]
pub async fn refunds_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::GlobalRefundId>,
    query_params: web::Query<api_models::refunds::RefundsRetrieveBody>,
) -> HttpResponse {
    let refund_request = refunds::RefundsRetrieveRequest {
        refund_id: path.into_inner(),
        force_sync: query_params.force_sync,
        merchant_connector_details: None,
        return_raw_connector_response: query_params.return_raw_connector_response,
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
            let platform = auth.clone().into();
            refund_retrieve_core_with_refund_id(state, platform, auth.profile, refund_request)
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            },
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow))]
pub async fn refunds_retrieve_with_gateway_creds(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::GlobalRefundId>,
    payload: web::Json<api_models::refunds::RefundsRetrievePayload>,
) -> HttpResponse {
    let flow = match payload.force_sync {
        Some(true) => Flow::RefundsRetrieveForceSync,
        _ => Flow::RefundsRetrieve,
    };

    tracing::Span::current().record("flow", flow.to_string());

    let refund_request = refunds::RefundsRetrieveRequest {
        refund_id: path.into_inner(),
        force_sync: payload.force_sync,
        merchant_connector_details: payload.merchant_connector_details.clone(),
        return_raw_connector_response: payload.return_raw_connector_response,
    };

    let auth_type = if state.conf.merchant_id_auth.merchant_id_auth_enabled {
        &auth::MerchantIdAuth
    } else {
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            },
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        )
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        refund_request,
        |state, auth: auth::AuthenticationData, refund_request, _| {
            let platform = auth.clone().into();
            refund_retrieve_core_with_refund_id(state, platform, auth.profile, refund_request)
        },
        auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
/// Refunds - Retrieve (POST)
///
/// To retrieve the properties of a Refund. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
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
            let platform = auth.clone().into();
            refund_response_wrapper(
                state,
                platform,
                auth.profile_id,
                req,
                refund_retrieve_core_with_refund_id,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
/// Refunds - Update
///
/// To update the properties of a Refund object. This may include attaching a reason for the refund or metadata fields
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
            let platform = auth.into();
            refund_update_core(state, platform, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::RefundsUpdate))]
pub async fn refunds_metadata_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<refunds::RefundMetadataUpdateRequest>,
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
            refund_metadata_update_core(
                state,
                auth.merchant_account,
                req.payload,
                global_refund_id.clone(),
            )
        },
        &auth::V2ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "olap"))]
/// Refunds - List
///
/// To list the refunds associated with a payment_id or with the merchant, if payment_id is not provided
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
            let platform = auth.into();
            refund_list(state, platform, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v2", feature = "olap"))]
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
            refund_list(state, auth.merchant_account, auth.profile, req)
        },
        auth::auth_type(
            &auth::V2ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            },
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "olap"))]
/// Refunds - List at profile level
///
/// To list the refunds associated with a payment_id or with the merchant, if payment_id is not provided
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
            let platform = auth.clone().into();
            refund_list(
                state,
                platform,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "olap"))]
/// Refunds - Filter
///
/// To list the refunds filters associated with list of connectors, currencies and payment statuses
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
            let platform = auth.into();
            refund_filter_list(state, platform, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "olap"))]
/// Refunds - Filter V2
///
/// To list the refunds filters associated with list of connectors, currencies and payment statuses
#[instrument(skip_all, fields(flow = ?Flow::RefundsFilters))]
pub async fn get_refunds_filters(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::RefundsFilters;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            let platform = auth.into();
            get_filters_for_refunds(state, platform, None)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "olap"))]
/// Refunds - Filter V2 at profile level
///
/// To list the refunds filters associated with list of connectors, currencies and payment statuses
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
            let platform = auth.clone().into();
            get_filters_for_refunds(
                state,
                platform,
                auth.profile_id.map(|profile_id| vec![profile_id]),
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "olap"))]
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
            let platform = auth.into();
            get_aggregates_for_refunds(state, platform, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::MerchantRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "olap"))]
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

#[cfg(all(feature = "v1", feature = "olap"))]
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
            let platform = auth.clone().into();
            get_aggregates_for_refunds(
                state,
                platform,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth {
                is_connected_allowed: false,
                is_platform_allowed: false,
            }),
            &auth::JWTAuth {
                permission: Permission::ProfileRefundRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
