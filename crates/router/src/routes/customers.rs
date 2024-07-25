#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use actix_web::Responder;
use actix_web::{web, HttpRequest, HttpResponse};
#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
use common_utils::id_type;
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
        |state, auth, req, _| create_customer(state, auth.merchant_account, auth.key_store, req),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
pub async fn customers_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
) -> HttpResponse {
    let flow = Flow::CustomersRetrieve;

    let payload = web::Json(customers::CustomerId::new_customer_id_struct(
        path.into_inner(),
    ))
    .into_inner();

    let auth = if auth::is_jwt_auth(req.headers()) {
        Box::new(auth::JWTAuth(Permission::CustomerRead))
    } else {
        match auth::is_ephemeral_auth(req.headers()) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        }
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req, _| retrieve_customer(state, auth.merchant_account, auth.key_store, req),
        &*auth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all, fields(flow = ?Flow::CustomersList))]
pub async fn customers_list(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::CustomersList;

    api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth, _, _| {
            list_customers(
                state,
                auth.merchant_account.get_id().to_owned(),
                auth.key_store,
            )
        },
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerRead),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    )
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
pub async fn customers_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
    mut json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersUpdate;
    let customer_id = path.into_inner();
    json_payload.customer_id = Some(customer_id);
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req, _| update_customer(state, auth.merchant_account, req, auth.key_store),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
pub async fn customers_delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
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
        |state, auth, req, _| delete_customer(state, auth.merchant_account, req, auth.key_store),
        auth::auth_type(
            &auth::ApiKeyAuth,
            &auth::JWTAuth(Permission::CustomerWrite),
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
#[instrument(skip_all, fields(flow = ?Flow::CustomersGetMandates))]
pub async fn get_customer_mandates(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<id_type::CustomerId>,
) -> impl Responder {
    let flow = Flow::CustomersGetMandates;
    let customer_id = customers::CustomerId {
        customer_id: path.into_inner(),
    };

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        customer_id,
        |state, auth, req, _| {
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
    ))
    .await
}
