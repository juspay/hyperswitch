use actix_web::{web, HttpRequest, HttpResponse, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::customers::*,
    services::{api, authentication as auth},
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
        &auth::ApiKeyAuth,
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

    let auth =
        match auth::is_ephemeral_auth(req.headers(), &*state.store, &payload.customer_id).await {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        };

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| retrieve_customer(&*state.store, merchant_account, req),
        &*auth,
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
        &auth::ApiKeyAuth,
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
    api::server_wrap(&state, &req, payload, delete_customer, &auth::ApiKeyAuth).await
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
        *auth::jwt_auth_or(&auth::ApiKeyAuth, req.headers()),
    )
    .await
}
