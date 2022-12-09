mod types;

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use error_stack::report;
use router_env::{tracing, tracing::instrument};

use crate::{
    compatibility::{stripe, wrap},
    core::payments,
    routes::AppState,
    services::api,
    types::api::{self as api_types, PSync, PaymentsRequest, PaymentsRetrieveRequest, Verify},
};

#[post("")]
#[instrument(skip_all)]
pub async fn setup_intents_create(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::StripeSetupIntentRequest = match qs_config.deserialize_bytes(&form_payload)
    {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
        }
    };

    let create_payment_req: PaymentsRequest = payload.into();

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        create_payment_req,
        |state, merchant_account, req| {
            payments::payments_core::<Verify, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCreate,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[get("/{setup_id}")]
pub async fn setup_intents_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = PaymentsRetrieveRequest {
        resource_id: api_types::PaymentIdType::PaymentIntentId(path.to_string()),
        merchant_id: None,
        force_sync: true,
        connector: None,
        param: None,
    };

    let auth_type = match api::get_auth_type(&req) {
        Ok(auth_type) => auth_type,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };
    let auth_flow = api::get_auth_flow(&auth_type);

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        payload,
        |state, merchant_account, payload| {
            payments::payments_core::<PSync, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentStatus,
                payload,
                auth_flow,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{setup_id}")]
pub async fn setup_intents_update(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let setup_id = path.into_inner();
    let stripe_payload: types::StripeSetupIntentRequest =
        match qs_config.deserialize_bytes(&form_payload) {
            Ok(p) => p,
            Err(err) => {
                return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
            }
        };

    let mut payload: PaymentsRequest = stripe_payload.into();
    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(setup_id));

    let auth_type;
    (payload, auth_type) = match api::get_auth_type_and_check_client_secret(&req, payload) {
        Ok(values) => values,
        Err(err) => return api::log_and_return_error_response(err),
    };
    let auth_flow = api::get_auth_flow(&auth_type);
    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<Verify, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentUpdate,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{setup_id}/confirm")]
pub async fn setup_intents_confirm(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let setup_id = path.into_inner();
    let stripe_payload: types::StripeSetupIntentRequest =
        match qs_config.deserialize_bytes(&form_payload) {
            Ok(p) => p,
            Err(err) => {
                return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
            }
        };

    let mut payload: PaymentsRequest = stripe_payload.into();
    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(setup_id));
    payload.confirm = Some(true);

    let auth_type;
    (payload, auth_type) = match api::get_auth_type_and_check_client_secret(&req, payload) {
        Ok(values) => values,
        Err(err) => return api::log_and_return_error_response(err),
    };
    let auth_flow = api::get_auth_flow(&auth_type);
    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<Verify, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentConfirm,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}
