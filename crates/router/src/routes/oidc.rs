use actix_web::{web, HttpRequest, HttpResponse};
use api_models::oidc as oidc_types;
use router_env::{instrument, tracing, Flow};

use crate::{
    core::api_locking,
    routes::app::AppState,
    services::{api, authentication as auth, oidc_provider},
};

/// OpenID Connect Discovery Document
#[instrument(skip_all, fields(flow = ?Flow::OidcDiscovery))]
pub async fn oidc_discovery(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    Box::pin(api::server_wrap(
        Flow::OidcDiscovery,
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
    Box::pin(api::server_wrap(
        Flow::OidcJwks,
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
    query_params: web::Query<oidc_types::OidcAuthorizeQuery>,
) -> HttpResponse {
    Box::pin(api::server_wrap(
        Flow::OidcAuthorize,
        state,
        &req,
        query_params.into_inner(),
        |state,
         user: Option<auth::UserFromToken>,
         req_payload: oidc_types::OidcAuthorizeQuery,
         _| oidc_provider::process_authorize_request(state, req_payload, user),
        auth::auth_type(
            &auth::NoAuth,
            &auth::DashboardNoPermissionAuth {
                allow_connected: true,
                allow_platform: true,
            },
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
    form_data: web::Form<oidc_types::OidcTokenRequest>,
) -> HttpResponse {
    Box::pin(api::server_wrap(
        Flow::OidcToken,
        state,
        &req,
        form_data.into_inner(),
        |state, client_id: String, req_body, _| {
            oidc_provider::process_token_request(state, req_body, client_id)
        },
        &auth::OIDC_CLIENT_AUTH,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
