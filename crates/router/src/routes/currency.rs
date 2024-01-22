use actix_web::{web, HttpRequest, HttpResponse};
use router_env::Flow;

use crate::{
    core::{api_locking, currency},
    routes::AppState,
    services::{api, authentication as auth, authorization::permissions::Permission},
};

pub async fn retrieve_forex(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::RetrieveForexFlow;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, _auth: auth::AuthenticationData, _| currency::retrieve_forex(state),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::ForexRead),
            req.headers(),
        ),
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
        |state, _, _| {
            currency::convert_forex(
                state,
                *amount,
                to_currency.to_string(),
                from_currency.to_string(),
            )
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::ForexRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
