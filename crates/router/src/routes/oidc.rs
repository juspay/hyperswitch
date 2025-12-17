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
