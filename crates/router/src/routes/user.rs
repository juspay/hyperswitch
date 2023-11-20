use actix_web::{web, HttpRequest, HttpResponse};
use api_models::user as user_api;
use router_env::Flow;

use super::AppState;
use crate::{
    core::{api_locking, user},
    services::{
        api,
        authentication::{self as auth},
    },
};

pub async fn user_signup(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignUpRequest>,
) -> HttpResponse {
    let flow = Flow::UserSignUp;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body| user::signup(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn user_signin(
    state: web::Data<AppState>,
    http_req: HttpRequest,
    json_payload: web::Json<user_api::SignInRequest>,
) -> HttpResponse {
    let flow = Flow::UserSignIn;
    let req_payload = json_payload.into_inner();
    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        req_payload.clone(),
        |state, _, req_body| user::signin(state, req_body),
        &auth::NoAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
