use actix_web::{web, HttpRequest, HttpResponse};
use api_models::pm_blacklist as pm_blacklist_model;
use router_env::Flow;

use crate::{
    core::{api_locking, pm_blacklist},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

pub async fn block_payment_method(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<pm_blacklist_model::BlacklistPmRequest>,
) -> HttpResponse {
    let flow = Flow::PmBlockFlow;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body| {
            pm_blacklist::block_payment_method(state, &req, body, auth.merchant_account)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MerchantAccountRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn unblock_payment_method (
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<pm_blacklist_model::UnblockPmRequest>,
) -> HttpResponse {
    let flow = Flow::PmBlockFlow;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body| {
            pm_blacklist::unblock_payment_method(state, &req, body, auth.merchant_account)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MerchantAccountRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_blocked_payment_methods (
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let flow = Flow::PmBlockFlow;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, body| {
            pm_blacklist::list_blocked_payment_methods(state, &req, auth.merchant_account)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MerchantAccountRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
