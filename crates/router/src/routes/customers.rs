use actix_web::{web, HttpRequest, HttpResponse, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, customers::*},
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api::customers,
};

#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
pub async fn customers_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersCreate;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req| create_customer(state, auth.merchant_account, auth.key_store, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
pub async fn customers_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let flow = Flow::CustomersRetrieve;
    let payload = web::Json(customers::CustomerId {
        customer_id: path.into_inner(),
    })
    .into_inner();

    let auth = if auth::is_jwt_auth(req.headers()) {
        Box::new(auth::JWTAuth(Permission::CustomerRead))
    } else {
        match auth::is_ephemeral_auth(req.headers(), &*state.store, &payload.customer_id).await {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        }
    };

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| retrieve_customer(state, auth.merchant_account, auth.key_store, req),
        &*auth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersList))]
pub async fn customers_list(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::CustomersList;

    api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth, _| list_customers(state, auth.merchant_account.merchant_id, auth.key_store),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
pub async fn customers_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    mut json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersUpdate;
    let customer_id = path.into_inner();
    json_payload.customer_id = customer_id;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req| update_customer(state, auth.merchant_account, req, auth.key_store),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
pub async fn customers_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::CustomersCreate;
    let payload = web::Json(customers::CustomerId {
        customer_id: path.into_inner(),
    })
    .into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| delete_customer(state, auth.merchant_account, req, auth.key_store),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::CustomersGetMandates))]
pub async fn get_customer_mandates(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> impl Responder {
    let flow = Flow::CustomersGetMandates;
    let customer_id = customers::CustomerId {
        customer_id: path.into_inner(),
    };

    api::server_wrap(
        flow,
        state,
        &req,
        customer_id,
        |state, auth, req| {
            crate::core::mandate::get_customer_mandates(
                state,
                auth.merchant_account,
                auth.key_store,
                req,
            )
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::MandateRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
