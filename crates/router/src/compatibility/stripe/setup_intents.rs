pub mod types;

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use api_models::payments as payment_types;
use error_stack::report;
use router_env::{instrument, tracing};

use crate::{
    compatibility::{stripe::errors, wrap},
    core::payments,
    routes,
    services::{api, authentication as auth},
    types::api as api_types,
};

#[post("")]
#[instrument(skip_all)]
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

    let create_payment_req: payment_types::PaymentsRequest = match payload.try_into() {
        Ok(req) => req,
        Err(err) => return api::log_and_return_error_response(err),
    };

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
    >(
        state.get_ref(),
        &req,
        create_payment_req,
        |state, merchant_account, req| {
            payments::payments_core::<api_types::Verify, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCreate,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all)]
#[get("/{setup_id}")]
pub async fn setup_intents_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: api_types::PaymentIdType::PaymentIntentId(path.to_string()),
        merchant_id: None,
        force_sync: true,
        connector: None,
        param: None,
        merchant_connector_details: None,
    };

    let (auth_type, auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
    >(
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, payload| {
            payments::payments_core::<api_types::PSync, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentStatus,
                payload,
                auth_flow,
                payments::CallConnectorAction::Trigger,
            )
        },
        &*auth_type,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{setup_id}")]
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

    let mut payload: payment_types::PaymentsRequest = match stripe_payload.try_into() {
        Ok(req) => req,
        Err(err) => return api::log_and_return_error_response(err),
    };
    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(setup_id));

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        };

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
    >(
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<api_types::Verify, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentUpdate,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
            )
        },
        &*auth_type,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{setup_id}/confirm")]
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

    let mut payload: payment_types::PaymentsRequest = match stripe_payload.try_into() {
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

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripeSetupIntentResponse,
        errors::StripeErrorCode,
    >(
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<api_types::Verify, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentConfirm,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
            )
        },
        &*auth_type,
    )
    .await
}
