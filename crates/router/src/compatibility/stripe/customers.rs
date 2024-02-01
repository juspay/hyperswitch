pub mod types;
use actix_web::{web, HttpRequest, HttpResponse};
use error_stack::report;
use router_env::{instrument, tracing, Flow};

use crate::{
    compatibility::{stripe::errors, wrap},
    core::{api_locking, customers, payment_methods::cards},
    routes,
    services::{api, authentication as auth},
    types::api::{customers as customer_types, payment_methods},
};

#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
/// Handles the creation of a new customer by deserializing the request payload, converting it into a customer request, and then using the compatibility API to create the customer. Returns a `HttpResponse` with the result of the operation.
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

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CreateCustomerResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        create_cust_req,
        |state, auth, req| {
            customers::create_customer(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::CustomersRetrieve))]
/// Retrieves a customer using the provided customer ID from the web::Data<routes::AppState> state
/// and HttpRequest req. It then constructs a payload using the provided path and creates a flow for
/// customer retrieval. The method then wraps the customer retrieval function in a compatibility_api_wrap
/// and awaits the result using Box::pin.
pub async fn customer_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = customer_types::CustomerId {
        customer_id: path.into_inner(),
    };

    let flow = Flow::CustomersRetrieve;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerRetrieveResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            customers::retrieve_customer(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::CustomersUpdate))]
/// Handles the update of a customer by deserializing the form payload into a CustomerUpdateRequest,
/// updating the customer with the provided customer ID, and then calling the update_customer function
/// to perform the update operation. Returns a response wrapped in a compatibility_api_wrap.
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

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerUpdateResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        cust_update_req,
        |state, auth, req| {
            customers::update_customer(state, auth.merchant_account, req, auth.key_store)
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::CustomersDelete))]
/// Handles the HTTP request to delete a customer. This method takes the application state, HTTP request,
/// and the path containing the customer ID. It creates a payload with the customer ID, initializes the flow
/// as CustomersDelete, and then uses compatibility_api_wrap to wrap the delete operation in a compatible
/// async block. The result is then awaited and returned as an HTTP response.
pub async fn customer_delete(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = customer_types::CustomerId {
        customer_id: path.into_inner(),
    };

    let flow = Flow::CustomersDelete;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerDeleteResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            customers::delete_customer(state, auth.merchant_account, req, auth.key_store)
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::CustomerPaymentMethodsList))]
/// This method is used to list the payment methods for a specific customer. It takes in the application state, the HTTP request, the customer ID, and a JSON payload containing the payment method list request. It then performs a compatibility API wrap, passing in the necessary parameters and handling the asynchronous operation. The method returns a HTTP response.
pub async fn list_customer_payment_method_api(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Query<payment_methods::PaymentMethodListRequest>,
) -> HttpResponse {
    let payload = json_payload.into_inner();
    let customer_id = path.into_inner();
    let flow = Flow::CustomerPaymentMethodsList;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::CustomerPaymentMethodListResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            cards::do_list_customer_pm_fetch_customer_if_not_passed(
                state,
                auth.merchant_account,
                auth.key_store,
                Some(req),
                Some(customer_id.as_str()),
            )
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
