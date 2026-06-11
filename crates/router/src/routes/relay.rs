use actix_web::{web, Responder};
use common_utils::ext_traits::OptionExt;
use error_stack::ResultExt;
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{api_locking, errors, relay},
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::RelayUnreferencedRefund))]
#[cfg(feature = "oltp")]
pub async fn unreferenced_refund(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<api_models::unreferenced_refund::UnreferencedRefundRequest>,
) -> impl Responder {
    let flow = Flow::RelayUnreferencedRefund;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| async move {
            #[cfg(feature = "v1")]
            let profile_id = auth
                .profile
                .get_required_value("profile_id")
                .change_context(errors::ApiErrorResponse::MissingRequiredField {
                    field_name: "profile_id",
                })?
                .get_id()
                .clone();
            #[cfg(feature = "v2")]
            let profile_id = auth.profile.get_id().clone();
            relay::relay_unreferenced_refund(state, auth.platform, profile_id, req).await
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            allow_connected_scope_operation: false,
            allow_platform_self_operation: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::Relay))]
#[cfg(feature = "oltp")]
pub async fn relay(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<api_models::relay::RelayRequest>,
) -> impl Responder {
    let flow = Flow::Relay;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            relay::relay_flow_decider(
                state,
                auth.platform,
                #[cfg(feature = "v1")]
                auth.profile.map(|profile| profile.get_id().clone()),
                #[cfg(feature = "v2")]
                Some(auth.profile.get_id().clone()),
                req,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::RelayRetrieve))]
#[cfg(feature = "oltp")]
pub async fn relay_retrieve(
    state: web::Data<app::AppState>,
    path: web::Path<common_utils::id_type::RelayId>,
    req: actix_web::HttpRequest,
    query_params: web::Query<api_models::relay::RelayRetrieveBody>,
) -> impl Responder {
    let flow = Flow::RelayRetrieve;
    let relay_retrieve_request = api_models::relay::RelayRetrieveRequest {
        force_sync: query_params.force_sync,
        id: path.into_inner(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        relay_retrieve_request,
        |state, auth: auth::AuthenticationData, req, _| {
            relay::relay_retrieve(
                state,
                auth.platform,
                #[cfg(feature = "v1")]
                auth.profile.map(|profile| profile.get_id().clone()),
                #[cfg(feature = "v2")]
                Some(auth.profile.get_id().clone()),
                req,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            allow_connected_scope_operation: true,
            allow_platform_self_operation: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
