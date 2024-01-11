use actix_web::{web, HttpRequest, HttpResponse};
use api_models::blocklist as api_blocklist;
use router_env::Flow;

use crate::{
    core::{api_locking, blocklist},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

pub async fn add_entry_to_blocklist(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_blocklist::AddToBlocklistRequest>,
) -> HttpResponse {
    let flow = Flow::AddToBlocklist;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body| {
            blocklist::add_entry_to_blocklist(state, auth.merchant_account, body)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MerchantAccountWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn remove_entry_from_blocklist(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<api_blocklist::DeleteFromBlocklistRequest>,
) -> HttpResponse {
    let flow = Flow::DeleteFromBlocklist;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationData, body| {
            blocklist::remove_entry_from_blocklist(state, auth.merchant_account, body)
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MerchantAccountWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn list_blocked_payment_methods(
    state: web::Data<AppState>,
    req: HttpRequest,
    query_payload: web::Query<api_blocklist::ListBlocklistQuery>,
) -> HttpResponse {
    let flow = Flow::ListBlocklist;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        query_payload.into_inner(),
        |state, auth: auth::AuthenticationData, query| {
            blocklist::list_blocklist_entries(state, auth.merchant_account, query)
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
