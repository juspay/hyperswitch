use actix_web::{web, HttpRequest, HttpResponse};
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use super::app::AppState;
use crate::{
    core::{
        errors::{http_not_implemented, BachResult},
        payment_methods::cards,
    },
    services::api,
    types::api::payment_methods::{self, PaymentMethodId},
};

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsCreate))]
// #[post("")]
pub async fn create_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<payment_methods::CreatePaymentMethod>,
) -> HttpResponse {
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| async move {
            let merchant_id = merchant_account.merchant_id.clone();

            cards::add_payment_method(state, req, merchant_id).await
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsList))]
//#[get("{merchant_id}")]
pub async fn list_payment_method_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    _merchant_id: web::Path<String>,
    json_payload: web::Query<payment_methods::ListPaymentMethodRequest>,
) -> HttpResponse {
    //let merchant_id = merchant_id.into_inner();
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| {
            cards::list_payment_methods(&state.store, merchant_account, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
// #[get("/{customer_id}/payment_methods")]
pub async fn list_customer_payment_method_api(
    state: web::Data<AppState>,
    customer_id: web::Path<(String,)>,
    req: HttpRequest,
    json_payload: web::Query<payment_methods::ListPaymentMethodRequest>,
) -> HttpResponse {
    let customer_id = customer_id.into_inner().0;
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, _| {
            cards::list_customer_payment_method(state, merchant_account, &customer_id)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsRetrieve))]
// #[get("/{payment_method_id}")]
pub async fn payment_method_retrieve_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = web::Json(PaymentMethodId {
        payment_method_id: path.into_inner(),
    })
    .into_inner();

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, _, pm| cards::retrieve_payment_method(state, pm),
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsUpdate))]
// #[post("/{payment_method_id}")]
pub async fn payment_method_update_api(
    _state: web::Data<AppState>,
    _req: HttpRequest,
    _path: web::Path<String>,
) -> BachResult<HttpResponse> {
    Ok(http_not_implemented())
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentMethodsDelete))]
// #[post("/{payment_method_id}/detach")]
pub async fn payment_method_delete_api(
    state: web::Data<AppState>,
    req: HttpRequest,
    payment_method_id: web::Path<(String,)>,
) -> HttpResponse {
    let pm = PaymentMethodId {
        payment_method_id: payment_method_id.into_inner().0,
    };
    api::server_wrap(
        &state,
        &req,
        pm,
        cards::delete_payment_method,
        api::MerchantAuthentication::ApiKey,
    )
    .await
}
