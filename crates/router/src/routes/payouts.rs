use actix_web::{
    body::{BoxBody, MessageBody},
    web, HttpRequest, HttpResponse, Responder,
};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, payouts::*},
    services::{
        api,
        authentication::{self as auth},
        authorization::permissions::Permission,
    },
    types::api::payouts as payout_types,
};

/// Payouts - Create
#[instrument(skip_all, fields(flow = ?Flow::PayoutsCreate))]
pub async fn payouts_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsCreate;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_create_core(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "v1", feature = "payouts"))]
/// Payouts - Retrieve
#[instrument(skip_all, fields(flow = ?Flow::PayoutsRetrieve))]
pub async fn payouts_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query_params: web::Query<payout_types::PayoutRetrieveBody>,
) -> HttpResponse {
    let payout_retrieve_request = payout_types::PayoutRetrieveRequest {
        payout_id: path.into_inner(),
        force_sync: query_params.force_sync.to_owned(),
        merchant_id: query_params.merchant_id.to_owned(),
    };
    let flow = Flow::PayoutsRetrieve;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payout_retrieve_request,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_retrieve_core(
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
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Update
#[instrument(skip_all, fields(flow = ?Flow::PayoutsUpdate))]
pub async fn payouts_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
) -> HttpResponse {
    let flow = Flow::PayoutsUpdate;
    let payout_id = path.into_inner();
    let mut payout_update_payload = json_payload.into_inner();
    payout_update_payload.payout_id = Some(payout_id);
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payout_update_payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_update_core(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsConfirm))]
pub async fn payouts_confirm(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutCreateRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PayoutsConfirm;
    let mut payload = json_payload.into_inner();
    let payout_id = path.into_inner();
    tracing::Span::current().record("payout_id", &payout_id);
    payload.payout_id = Some(payout_id);
    payload.confirm = Some(true);
    let (auth_type, _auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(e) => return api::log_and_return_error_response(e),
        };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req, _| {
            payouts_confirm_core(state, auth.merchant_account, auth.key_store, req)
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Cancel
#[instrument(skip_all, fields(flow = ?Flow::PayoutsCancel))]
pub async fn payouts_cancel(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutActionRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PayoutsCancel;
    let mut payload = json_payload.into_inner();
    payload.payout_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_cancel_core(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
/// Payouts - Fulfill
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFulfill))]
pub async fn payouts_fulfill(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutActionRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::PayoutsFulfill;
    let mut payload = json_payload.into_inner();
    payload.payout_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_fulfill_core(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - List
#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Query<payout_types::PayoutListConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_list_core(state, auth.merchant_account, None, auth.key_store, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantPayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - List Profile
#[cfg(all(feature = "olap", feature = "payouts", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Query<payout_types::PayoutListConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_list_core(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                auth.key_store,
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Filtered list
#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list_by_filter(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutListFilterConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_filtered_list_core(state, auth.merchant_account, None, auth.key_store, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantPayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Filtered list
#[cfg(all(feature = "olap", feature = "payouts", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsList))]
pub async fn payouts_list_by_filter_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payout_types::PayoutListFilterConstraints>,
) -> HttpResponse {
    let flow = Flow::PayoutsList;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_filtered_list_core(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                auth.key_store,
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Available filters for Merchant
#[cfg(feature = "olap")]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFilter))]
pub async fn payouts_list_available_filters_for_merchant(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<common_utils::types::TimeRange>,
) -> HttpResponse {
    let flow = Flow::PayoutsFilter;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_list_available_filters_core(state, auth.merchant_account, None, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::MerchantPayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payouts - Available filters for Profile
#[cfg(all(feature = "olap", feature = "payouts", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PayoutsFilter))]
pub async fn payouts_list_available_filters_for_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<common_utils::types::TimeRange>,
) -> HttpResponse {
    let flow = Flow::PayoutsFilter;
    let payload = json_payload.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payouts_list_available_filters_core(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::ProfilePayoutRead,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PayoutsAccounts))]
// #[get("/accounts")]
pub async fn payouts_accounts() -> impl Responder {
    let _flow = Flow::PayoutsAccounts;
    http_response("accounts")
}

fn http_response<T: MessageBody + 'static>(response: T) -> HttpResponse<BoxBody> {
    HttpResponse::Ok().body(response)
}
