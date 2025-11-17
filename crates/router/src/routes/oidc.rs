use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::api_locking,
    routes::app::AppState,
    services::{api, authentication as auth, oidc_provider},
};

/// OpenID Connect Discovery Document
#[instrument(skip_all, fields(flow = ?Flow::OidcDiscovery))]
pub async fn oidc_discovery(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::OidcDiscovery;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _: (), _, _| oidc_provider::get_discovery_document(state),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// JWKS Endpoint - Exposes public keys for ID token verification
#[instrument(skip_all, fields(flow = ?Flow::OidcJwks))]
pub async fn jwks_endpoint(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::OidcJwks;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _: (), _, _| oidc_provider::get_jwks(state),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::OidcAuthorize))]
pub async fn oidc_authorize(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_models::oidc::OidcAuthorizeQuery>,
) -> HttpResponse {
    let flow = Flow::OidcAuthorize;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state,
         user: Option<auth::UserFromToken>,
         req_body: api_models::oidc::OidcAuthorizeQuery,
         _| { oidc_provider::process_authorize_request(state, req_body, user) },
        auth::auth_type(
            &auth::NoAuth,
            &auth::DashboardNoPermissionAuth,
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::OidcToken))]
pub async fn oidc_token(
    state: web::Data<AppState>,
    req: HttpRequest,
    form_data: web::Form<api_models::oidc::OidcTokenRequest>,
) -> HttpResponse {
    let flow = Flow::OidcToken;
    let headers = req.headers().clone();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        form_data.into_inner(),
        move |state, _: (), req_body, _| {
            let headers = headers.clone();
            oidc_provider::process_token_request(state, req_body, headers)
        },
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
