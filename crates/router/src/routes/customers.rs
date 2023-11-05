use actix_web::{web, HttpRequest, HttpResponse, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, customers::*},
    services::{api, authentication as auth},
    types::api::customers,
};

/// Create Customer
///
/// Create a customer object and store the customer details to be reused for future payments. Incase the customer already exists in the system, this API will respond with the customer details.
#[utoipa::path(
    post,
    path = "/customers",
    request_body = CustomerRequest,
    responses(
        (status = 200, description = "Customer Created", body = CustomerResponse),
        (status = 400, description = "Invalid data")
    ),
    tag = "Customers",
    operation_id = "Create a Customer",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
pub async fn customers_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<customers::CustomerRequest>,
) -> HttpResponse {
    let flow = Flow::CustomersCreate;
    api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req| create_customer(state, auth.merchant_account, auth.key_store, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Retrieve Customer
///
/// Retrieve a customer's details.
#[utoipa::path(
    get,
    path = "/customers/{customer_id}",
    params (("customer_id" = String, Path, description = "The unique identifier for the Customer")),
    responses(
        (status = 200, description = "Customer Retrieved", body = CustomerResponse),
        (status = 404, description = "Customer was not found")
    ),
    tag = "Customers",
    operation_id = "Retrieve a Customer",
    security(("api_key" = []), ("ephemeral_key" = []))
)]
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

    let auth =
        match auth::is_ephemeral_auth(req.headers(), &*state.store, &payload.customer_id).await {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
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

/// List customers for a merchant
///
/// To filter and list the customers for a particular merchant id
#[utoipa::path(
    post,
    path = "/customers/list",
    responses(
        (status = 200, description = "Customers retrieved", body = Vec<CustomerResponse>),
        (status = 400, description = "Invalid Data"),
    ),
    tag = "Customers List",
    operation_id = "List all Customers for a Merchant",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::CustomersList))]
pub async fn customers_list(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let flow = Flow::CustomersList;

    api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth, _| list_customers(state, auth.merchant_account.merchant_id, auth.key_store),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Update Customer
///
/// Updates the customer's details in a customer object.
#[utoipa::path(
    post,
    path = "/customers/{customer_id}",
    request_body = CustomerRequest,
    params (("customer_id" = String, Path, description = "The unique identifier for the Customer")),
    responses(
        (status = 200, description = "Customer was Updated", body = CustomerResponse),
        (status = 404, description = "Customer was not found")
    ),
    tag = "Customers",
    operation_id = "Update a Customer",
    security(("api_key" = []))
)]
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
    api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth, req| update_customer(state, auth.merchant_account, req, auth.key_store),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
/// Delete Customer
///
/// Delete a customer record.
#[utoipa::path(
    delete,
    path = "/customers/{customer_id}",
    params (("customer_id" = String, Path, description = "The unique identifier for the Customer")),
    responses(
        (status = 200, description = "Customer was Deleted", body = CustomerDeleteResponse),
        (status = 404, description = "Customer was not found")
    ),
    tag = "Customers",
    operation_id = "Delete a Customer",
    security(("api_key" = []))
)]
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
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| delete_customer(state, auth.merchant_account, req, auth.key_store),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
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
            crate::core::mandate::get_customer_mandates(state, auth.merchant_account, req)
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
