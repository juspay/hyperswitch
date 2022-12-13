mod types;

use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};
use error_stack::report;
use router_env::{tracing, tracing::instrument};

use crate::{
    compatibility::{stripe, wrap},
    core::customers,
    routes::AppState,
    services::api,
    types::api::customers as customer_types,
};

#[instrument(skip_all)]
#[post("")]
pub async fn customer_create(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::CreateCustomerRequest = match qs_config.deserialize_bytes(&form_payload) {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
        }
    };

    let create_cust_req: customer_types::CustomerRequest = payload.into();

    wrap::compatibility_api_wrap::<_, _, _, _, _, types::CreateCustomerResponse, stripe::ErrorCode>(
        &state,
        &req,
        create_cust_req,
        |state, merchant_account, req| {
            customers::create_customer(&*state.store, merchant_account, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[get("/{customer_id}")]
pub async fn customer_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = customer_types::CustomerId {
        customer_id: path.into_inner(),
    };

    wrap::compatibility_api_wrap::<_, _, _, _, _, types::CustomerRetrieveResponse, stripe::ErrorCode>(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            customers::retrieve_customer(&*state.store, merchant_account, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{customer_id}")]
pub async fn customer_update(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    path: web::Path<String>,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::CustomerUpdateRequest = match qs_config.deserialize_bytes(&form_payload) {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
        }
    };

    let customer_id = path.into_inner();
    let mut cust_update_req: customer_types::CustomerRequest = payload.into();
    cust_update_req.customer_id = customer_id;

    wrap::compatibility_api_wrap::<_, _, _, _, _, types::CustomerUpdateResponse, stripe::ErrorCode>(
        &state,
        &req,
        cust_update_req,
        |state, merchant_account, req| {
            customers::update_customer(&*state.store, merchant_account, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[delete("/{customer_id}")]
pub async fn customer_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = customer_types::CustomerId {
        customer_id: path.into_inner(),
    };

    wrap::compatibility_api_wrap::<_, _, _, _, _, types::CustomerDeleteResponse, stripe::ErrorCode>(
        &state,
        &req,
        payload,
        customers::delete_customer,
        api::MerchantAuthentication::ApiKey,
    )
    .await
}
