use actix_web::{web, HttpRequest, Responder};
use api_models::authentication::AuthenticationCreateRequest;
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
