pub mod types;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::payments as payment_types;
use error_stack::report;
use router_env::{instrument, tracing, Flow};

use crate::{
    compatibility::{
        stripe::{errors, payment_intents::types as stripe_payment_types},
        wrap,
    },
    core::{api_locking, payment_methods::Oss, payments},
    routes,
    services::{api, authentication as auth},
    types::api as api_types,
};

#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreate))]
/// This method is used to create a setup intent for a payment. It takes in the application state, query string configuration, HTTP request, and form payload, and then deserializes the payload into a Stripe setup intent request. It then creates a payment request from the setup intent, sets the flow to PaymentsCreate, and wraps the compatibility API to handle the setup intent creation. The method returns a response with the setup intent response or an error code if the creation fails.
pub async fn setup_intents_create(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::StripeSetupIntentRequest = match qs_config.deserialize_bytes(&form_payload)
    {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let create_payment_req: payment_types::PaymentsRequest =
        match payment_types::PaymentsRequest::try_from(payload) {
            Ok(req) => req,
            Err(err) => return api::log_and_return_error_response(err),
        };

    let flow = Flow::PaymentsCreate;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        create_payment_req,
        |state, auth, req| {
            payments::payments_core::<
                api_types::SetupMandate,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                Oss,
            >(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentCreate,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                api_types::HeaderPayload::default(),
            )
        },
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsRetrieve))]
/// Retrieves a setup intent from Stripe using the provided client secret and payment intent ID. 
/// 
/// # Arguments
/// * `state` - The web application state
/// * `req` - The HTTP request
/// * `path` - The path parameter containing the payment intent ID
/// * `query_payload` - The query parameter containing the client secret
/// 
/// # Returns
/// The HTTP response containing the retrieved setup intent data
pub async fn setup_intents_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query_payload: web::Query<stripe_payment_types::StripePaymentRetrieveBody>,
) -> HttpResponse {
    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: api_types::PaymentIdType::PaymentIntentId(path.to_string()),
        merchant_id: None,
        force_sync: true,
        connector: None,
        param: None,
        merchant_connector_details: None,
        client_secret: query_payload.client_secret.clone(),
        expand_attempts: None,
        expand_captures: None,
    };

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(report!(err)),
        };

    let flow = Flow::PaymentsRetrieve;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, payload| {
            payments::payments_core::<api_types::PSync, api_types::PaymentsResponse, _, _, _, Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentStatus,
                payload,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                api_types::HeaderPayload::default(),
            )
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsUpdate))]
/// This method handles the update of a setup intent for a payment. It takes in the application state,
/// query string configuration, HTTP request, form payload, and the path. It deserializes the form payload
/// into a Stripe setup intent request and creates a payment request from it. It then checks the client
/// secret and gets the authentication details from the request headers. After that, it calls the payments
/// core method to update the payment using the provided data. Finally, it returns the result as a
/// `types::StripeSetupIntentResponse` or an error of type `errors::StripeErrorCode` wrapped in a
/// compatibility API response.
pub async fn setup_intents_update(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let setup_id = path.into_inner();
    let stripe_payload: types::StripeSetupIntentRequest = match qs_config
        .deserialize_bytes(&form_payload)
    {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let mut payload: payment_types::PaymentsRequest =
        match payment_types::PaymentsRequest::try_from(stripe_payload) {
            Ok(req) => req,
            Err(err) => return api::log_and_return_error_response(err),
        };
    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(setup_id));

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        };

    let flow = Flow::PaymentsUpdate;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            payments::payments_core::<
                api_types::SetupMandate,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                Oss,
            >(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentUpdate,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                api_types::HeaderPayload::default(),
            )
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsConfirm))]
/// This method is used to confirm a setup intent for a payment. It takes in the application state, query string configuration, HTTP request, form payload, and the path as parameters. It deserializes the form payload into a Stripe setup intent request, creates a payment request from the deserialized payload, sets the payment ID and confirmation status, checks the client secret and gets the authentication type and flow, and then calls the payments_core method to confirm the payment. It returns a Stripe setup intent response or an error response based on the result of the payment confirmation process.
pub async fn setup_intents_confirm(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let setup_id = path.into_inner();
    let stripe_payload: types::StripeSetupIntentRequest = match qs_config
        .deserialize_bytes(&form_payload)
    {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let mut payload: payment_types::PaymentsRequest =
        match payment_types::PaymentsRequest::try_from(stripe_payload) {
            Ok(req) => req,
            Err(err) => return api::log_and_return_error_response(err),
        };
    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(setup_id));
    payload.confirm = Some(true);

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        };

    let flow = Flow::PaymentsConfirm;

    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            payments::payments_core::<
                api_types::SetupMandate,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                Oss,
            >(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentConfirm,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                api_types::HeaderPayload::default(),
            )
        },
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
