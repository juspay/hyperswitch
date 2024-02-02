use actix_web::{web, HttpRequest, HttpResponse, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{api_locking, customers::*},
    services::{api, authentication as auth, authorization::permissions::Permission},
    types::api::customers,
};

#[instrument(skip_all, fields(flow = ?Flow::CustomersCreate))]
/// Handles the creation of a new customer by processing the JSON payload and calling the create_customer function with the appropriate state and authentication parameters. The function also performs API server wrapping, authentication type verification, and locking action handling before returning the HTTP response.
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
/// Handles the retrieval of customer data. It first creates a JSON payload from the customer ID extracted from the request path. Then, it checks the authentication method used in the request and determines the appropriate authorization. Finally, it wraps the retrieval of customer data in a server_wrap function, passing the necessary parameters and awaits the result.
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
/// Asynchronously handles the HTTP request for listing customers. It wraps the flow in the API server, authenticates the request, and then awaits the response.
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
/// Asynchronously handles the update of a customer by processing the incoming HTTP request and JSON payload. 
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
/// This method handles the deletion of a customer by making an asynchronous call to the server_wrap function with the specified flow, state, request, payload, and delete_customer function. It also performs authentication using the ApiKeyAuth and JWTAuth with the specified permissions, as well as applying the NotApplicable lock action. The method returns an implementation of Responder.
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
/// Asynchronously retrieves the mandates for a specific customer by calling the appropriate API endpoint. It uses the provided `AppState` and `HttpRequest` to make the API call and retrieve the customer mandates based on the `customer_id` extracted from the `path`. The method also performs authentication using the `auth::ApiKeyAuth` and `auth::JWTAuth` with the required permission `Permission::MandateRead`. After the successful authentication and API call, it returns the result as a Responder.
pub async fn get_customer_mandates(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
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
    ))
    .await
}
