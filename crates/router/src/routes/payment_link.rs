use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, payment_link::*},
    services::{api, authentication as auth},
    AppState,
};

/// Payments Link - Retrieve
///
/// To retrieve the properties of a Payment Link. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/payment_link/{payment_link_id}",
    params(
        ("payment_link_id" = String, Path, description = "The identifier for payment link")
    ),
    request_body=RetrievePaymentLinkRequest,
    responses(
        (status = 200, description = "Gets details regarding payment link", body = RetrievePaymentLinkResponse),
        (status = 404, description = "No payment link found")
    ),
    tag = "Payments",
    operation_id = "Retrieve a Payment Link",
    security(("api_key" = []), ("publishable_key" = []))
)]
#[instrument(skip(state, req), fields(flow = ?Flow::PaymentLinkRetrieve))]

/// This method is used to retrieve a payment link. It takes in the application state, the HTTP request,
/// the path containing the payment link ID, and the JSON payload containing the request parameters.
/// It first verifies the client secret and authentication, then calls the retrieve_payment_link function
/// to retrieve the payment link and returns the response.
pub async fn payment_link_retrieve(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    json_payload: web::Query<api_models::payments::RetrievePaymentLinkRequest>,
) -> impl Responder {
    let flow = Flow::PaymentLinkRetrieve;
    let payload = json_payload.into_inner();
    let (auth_type, _) = match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
    };
    api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, _auth, _| retrieve_payment_link(state, path.clone()),
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Initiates a payment link for a specific merchant and payment ID.
pub async fn initiate_payment_link(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::PaymentLinkInitiate;
    let (merchant_id, payment_id) = path.into_inner();
    let payload = api_models::payments::PaymentLinkInitiateRequest {
        payment_id,
        merchant_id: merchant_id.clone(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth, _| {
            intiate_payment_link_flow(
                state,
                auth.merchant_account,
                payload.merchant_id.clone(),
                payload.payment_id.clone(),
            )
        },
        &crate::services::authentication::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payment Link - List
///
/// To list the payment links
#[utoipa::path(
    get,
    path = "/payment_link/list",
    params(
        ("limit" = Option<i64>, Query, description = "The maximum number of payment_link Objects to include in the response"),
        ("connector" = Option<String>, Query, description = "The connector linked to payment_link"),
        ("created_time" = Option<PrimitiveDateTime>, Query, description = "The time at which payment_link is created"),
        ("created_time.lt" = Option<PrimitiveDateTime>, Query, description = "Time less than the payment_link created time"),
        ("created_time.gt" = Option<PrimitiveDateTime>, Query, description = "Time greater than the payment_link created time"),
        ("created_time.lte" = Option<PrimitiveDateTime>, Query, description = "Time less than or equals to the payment_link created time"),
        ("created_time.gte" = Option<PrimitiveDateTime>, Query, description = "Time greater than or equals to the payment_link created time"),
    ),
    responses(
        (status = 200, description = "The payment link list was retrieved successfully"),
        (status = 401, description = "Unauthorized request")
    ),
    tag = "Payment Link",
    operation_id = "List all Payment links",
    security(("api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentLinkList))]
/// This method handles the HTTP request for listing payment links. It takes the Appstate, HttpRequest, and PaymentLinkListConstraints as input parameters and returns an implementation of Responder trait. Inside the method, it creates a flow for PaymentLinkList, extracts the payload, and then calls the server_wrap function from the api module to handle the request asynchronously. The server_wrap function passes the flow, state, request, payload, a closure for listing payment links, the API key authentication method, and locking action as parameters. Finally, it awaits the result and returns it.
pub async fn payments_link_list(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<api_models::payments::PaymentLinkListConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentLinkList;
    let payload = payload.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, payload| list_payment_link(state, auth.merchant_account, payload),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

/// Retrieves the status of a payment link by making an async request to the server. 
pub async fn payment_link_status(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::PaymentLinkStatus;
    let (merchant_id, payment_id) = path.into_inner();
    let payload = api_models::payments::PaymentLinkInitiateRequest {
        payment_id,
        merchant_id: merchant_id.clone(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth, _| {
            get_payment_link_status(
                state,
                auth.merchant_account,
                payload.merchant_id.clone(),
                payload.payment_id.clone(),
            )
        },
        &crate::services::authentication::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
