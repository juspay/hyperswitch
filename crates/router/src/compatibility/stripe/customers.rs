pub mod types;

use actix_web::{web, HttpRequest, HttpResponse};
use error_stack::report;
use router_env::{instrument, tracing, Flow};

use crate::{
    compatibility::{stripe::errors, wrap},
    core::{customers, payment_methods::cards},
    routes,
    services::{api, authentication as auth},
    types::api::customers as customer_types,
};

#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
pub async fn customer_create(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::CreateCustomerRequest = match qs_config.deserialize_bytes(&form_payload) {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let create_cust_req: customer_types::CustomerRequest = payload.into();

    let flow = Flow::CustomersCreate;

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CreateCustomerResponse,
        errors::StripeErrorCode,
    >(
        flow,
        state.get_ref(),
        &req,
        create_cust_req,
        |state, auth, req| {
            customers::create_customer(&*state.store, auth.merchant_account, auth.key_store, req)
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
pub async fn customer_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = customer_types::CustomerId {
        customer_id: path.into_inner(),
    };

    let flow = Flow::CustomersRetrieve;

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerRetrieveResponse,
        errors::StripeErrorCode,
    >(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, auth, req| {
            customers::retrieve_customer(&*state.store, auth.merchant_account, auth.key_store, req)
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
pub async fn customer_update(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    path: web::Path<String>,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::CustomerUpdateRequest = match qs_config.deserialize_bytes(&form_payload) {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let customer_id = path.into_inner();
    let mut cust_update_req: customer_types::CustomerRequest = payload.into();
    cust_update_req.customer_id = customer_id;

    let flow = Flow::CustomersUpdate;

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerUpdateResponse,
        errors::StripeErrorCode,
    >(
        flow,
        state.get_ref(),
        &req,
        cust_update_req,
        |state, auth, req| {
            customers::update_customer(&*state.store, auth.merchant_account, req, auth.key_store)
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
pub async fn customer_delete(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = customer_types::CustomerId {
        customer_id: path.into_inner(),
    };

    let flow = Flow::CustomersDelete;

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerDeleteResponse,
        errors::StripeErrorCode,
    >(
        flow,
        state.get_ref(),
        &req,
        payload,
        |state, auth, req| {
            customers::delete_customer(state, auth.merchant_account, req, auth.key_store)
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
pub async fn list_customer_payment_method_api(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let customer_id = path.into_inner();

    let flow = Flow::CustomerPaymentMethodsList;

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerPaymentMethodListResponse,
        errors::StripeErrorCode,
    >(
        flow,
        state.get_ref(),
        &req,
        customer_id.as_ref(),
        |state, auth, req| {
            cards::list_customer_payment_method(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::ApiKeyAuth,
    )
    .await
}
