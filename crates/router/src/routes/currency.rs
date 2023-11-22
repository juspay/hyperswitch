use actix_web::{web, HttpRequest, HttpResponse};
use router_env::Flow;
use crate::{
    core::{currency, api_locking},
    routes::AppState,
    services::{api, authentication as auth},
};

pub async fn retrieve_forex(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::RetrieveForexFlow;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _auth: auth::AuthenticationData, _| currency::retrieve_forex(state),
        #[cfg(not(feature = "release"))]
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
        #[cfg(feature = "release")]
        &auth::JWTAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn convert_forex(
    state: web::Data<AppState>,
    req: HttpRequest,
    params: web::Query<api_models::currency::CurrencyConversionParams>,
) -> HttpResponse {
    let flow = Flow::RetrieveForexFlow;
    let amount = &params.amount;
    let to_currency = &params.to_currency;
    let from_currency = &params.from_currency;
    Box::pin(api::server_wrap(
        flow,
        state.clone(),
        &req,
        (),
        |state, _auth: auth::AuthenticationData, _| {
            currency::convert_forex(
                state,
                *amount,
                to_currency.to_string(),
                from_currency.to_string(),
            )
        },
        #[cfg(not(feature = "release"))]
        auth::auth_type(&auth::ApiKeyAuth, &auth::JWTAuth, req.headers()),
        #[cfg(feature = "release")]
        &auth::JWTAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

