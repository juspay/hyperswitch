use actix_web::{web, HttpRequest, HttpResponse, Responder};
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use super::app::AppState;
use crate::{
    core::customers::*,
    services::{self, api},
    types::api::customers,
};

#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
// #[post("")]
pub async fn customers_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| create_customer(&*state.store, merchant_account, req),
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
// #[get("/{customer_id}")]
pub async fn customers_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = web::Json(customers::CustomerId {
        customer_id: path.into_inner(),
    })
    .into_inner();
    let auth_type = match services::authenticate_eph_key(
        &req,
        &*state.store,
        payload.customer_id.clone(),
    )
    .await
    {
        Ok(auth_type) => auth_type,
        Err(err) => return api::log_and_return_error_response(err),
    };
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| retrieve_customer(&*state.store, merchant_account, req),
        auth_type,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
// #[post("/{customer_id}")]
pub async fn customers_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    mut json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    let customer_id = path.into_inner();
    json_payload.customer_id = customer_id;
    api::server_wrap(
        &state,
        &req,
        json_payload.into_inner(),
        |state, merchant_account, req| update_customer(&*state.store, merchant_account, req),
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
// #[delete("/{customer_id}")]
pub async fn customers_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let payload = web::Json(customers::CustomerId {
        customer_id: path.into_inner(),
    })
    .into_inner();
    api::server_wrap(
        &state,
        &req,
        payload,
        delete_customer,
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersGetMandates))]
// #[get("/{customer_id}/mandates")]
pub async fn get_customer_mandates(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let customer_id = customers::CustomerId {
        customer_id: path.into_inner(),
    };

    api::server_wrap(
        &state,
        &req,
        customer_id,
        |state, merchant_account, req| {
            crate::core::mandate::get_customer_mandates(state, merchant_account, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}
