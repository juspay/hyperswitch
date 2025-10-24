use actix_web::{web, HttpRequest, Responder};
use api_models::authentication::{AuthenticationAuthenticateRequest, AuthenticationCreateRequest};
#[cfg(feature = "v1")]
use api_models::authentication::{
    AuthenticationEligibilityRequest, AuthenticationSyncPostUpdateRequest,
    AuthenticationSyncRequest,
};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, unified_authentication_service},
    routes::app::{self},
    services::{api, authentication as auth},
    types::domain,
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::AuthenticationCreate))]
pub async fn authentication_create(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    json_payload: web::Json<AuthenticationCreateRequest>,
) -> impl Responder {
    let flow = Flow::AuthenticationCreate;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            unified_authentication_service::authentication_create_core(state, merchant_context, req)
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
#[instrument(skip_all, fields(flow = ?Flow::AuthenticationEligibility))]
pub async fn authentication_eligibility(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    json_payload: web::Json<AuthenticationEligibilityRequest>,
    path: web::Path<common_utils::id_type::AuthenticationId>,
) -> impl Responder {
    let flow = Flow::AuthenticationEligibility;

    let api_auth = auth::ApiKeyAuth::default();
    let payload = json_payload.into_inner();

    let (auth, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth)
    {
        Ok((auth, _auth_flow)) => (auth, _auth_flow),
        Err(e) => return api::log_and_return_error_response(e),
    };

    let authentication_id = path.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            unified_authentication_service::authentication_eligibility_core(
                state,
                merchant_context,
                req,
                authentication_id.clone(),
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::AuthenticationAuthenticate))]
pub async fn authentication_authenticate(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    json_payload: web::Json<AuthenticationAuthenticateRequest>,
    path: web::Path<common_utils::id_type::AuthenticationId>,
) -> impl Responder {
    let flow = Flow::AuthenticationAuthenticate;
    let authentication_id = path.into_inner();
    let api_auth = auth::ApiKeyAuth::default();
    let payload = AuthenticationAuthenticateRequest {
        authentication_id,
        ..json_payload.into_inner()
    };

    let (auth, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
            Ok((auth, auth_flow)) => (auth, auth_flow),
            Err(e) => return api::log_and_return_error_response(e),
        };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            unified_authentication_service::authentication_authenticate_core(
                state,
                merchant_context,
                req,
                auth_flow,
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::AuthenticationSync))]
pub async fn authentication_sync(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::AuthenticationId,
    )>,
    json_payload: web::Query<AuthenticationSyncRequest>,
) -> impl Responder {
    let flow = Flow::AuthenticationSync;
    let api_auth = auth::ApiKeyAuth::default();
    let (_merchant_id, authentication_id) = path.into_inner();
    let payload = AuthenticationSyncRequest {
        authentication_id,
        ..json_payload.into_inner()
    };
    let (auth, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
            Ok((auth, auth_flow)) => (auth, auth_flow),
            Err(e) => return api::log_and_return_error_response(e),
        };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            unified_authentication_service::authentication_sync_core(
                state,
                merchant_context,
                auth_flow,
                req,
            )
        },
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::AuthenticationSyncPostUpdate))]
pub async fn authentication_sync_post_update(
    state: web::Data<app::AppState>,
    req: HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::AuthenticationId,
    )>,
) -> impl Responder {
    let flow = Flow::AuthenticationSyncPostUpdate;
    let (merchant_id, authentication_id) = path.into_inner();
    let payload = AuthenticationSyncPostUpdateRequest { authentication_id };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let merchant_context = domain::MerchantContext::NormalMerchant(Box::new(
                domain::Context(auth.merchant_account, auth.key_store),
            ));
            unified_authentication_service::authentication_post_sync_core(
                state,
                merchant_context,
                req,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
