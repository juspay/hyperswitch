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
