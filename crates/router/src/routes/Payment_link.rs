
use actix_web::{web, Responder};
use error_stack::report;
use router_env::{Flow};

use crate::{
    self as app,
    core::{
        errors::http_not_implemented,
        payments::{self, PaymentRedirectFlow},
    },
    openapi::examples::{
        PAYMENTS_CREATE, PAYMENTS_CREATE_MINIMUM_FIELDS, PAYMENTS_CREATE_WITH_ADDRESS,
        PAYMENTS_CREATE_WITH_CUSTOMER_DATA, PAYMENTS_CREATE_WITH_FORCED_3DS,
        PAYMENTS_CREATE_WITH_MANUAL_CAPTURE, PAYMENTS_CREATE_WITH_NOON_ORDER_CATETORY,
        PAYMENTS_CREATE_WITH_ORDER_DETAILS,
    },
    services::{api, authentication as auth},
    types::{
        api::{self as api_types, enums as api_enums, payments as payment_types},
        domain,
    },
};


pub async fn get_payment_link(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    query_payload: web::Query<api_models::payments::PaymentLinkRequest>,
) -> impl Responder {
    let flow = Flow::PaymentLinkRetrive;

    let payload = query_payload.into_inner();
    println!("payload {:?}", payload);

    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        payload.clone(),
        |state, auth, req| {
            payments::retrieve_payment_link(
                state,
                auth.merchant_account,
                auth.key_store,
                payload.payment_id.clone().unwrap(),
            )
        },
        &auth::ApiKeyAuth,
    )
    .await
}