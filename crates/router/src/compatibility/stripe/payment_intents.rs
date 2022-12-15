mod types;

use actix_web::{get, post, web, HttpRequest, HttpResponse};
use error_stack::report;
use router_env::{tracing, tracing::instrument};

use crate::{
    compatibility::{stripe, wrap},
    core::payments,
    routes::AppState,
    services::api,
    types::api::{
        self as api_types, payments::PaymentsCaptureRequest, Authorize, Capture, PSync,
        PaymentListConstraints, PaymentsCancelRequest, PaymentsRequest, PaymentsRetrieveRequest,
        Void,
    },
};

#[post("")]
#[instrument(skip_all)]
pub async fn payment_intents_create(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::StripePaymentIntentRequest =
        match qs_config.deserialize_bytes(&form_payload) {
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
        types::StripePaymentIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        create_payment_req,
        |state, merchant_account, req| {
            let connector = req.connector;
            payments::payments_core::<Authorize, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCreate,
                req,
                api::AuthFlow::Merchant,
                connector,
                payments::CallConnectorAction::Trigger,
            )
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[get("/{payment_id}")]
pub async fn payment_intents_retrieve(
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
        types::StripePaymentIntentResponse,
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
                None,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{payment_id}")]
pub async fn payment_intents_update(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let payment_id = path.into_inner();
    let stripe_payload: types::StripePaymentIntentRequest =
        match qs_config.deserialize_bytes(&form_payload) {
            Ok(p) => p,
            Err(err) => {
                return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
            }
        };

    let mut payload: PaymentsRequest = stripe_payload.into();
    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(payment_id));

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
        types::StripePaymentIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            let connector = req.connector;
            payments::payments_core::<Authorize, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentUpdate,
                req,
                auth_flow,
                connector,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{payment_id}/confirm")]
pub async fn payment_intents_confirm(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let payment_id = path.into_inner();
    let stripe_payload: types::StripePaymentIntentRequest =
        match qs_config.deserialize_bytes(&form_payload) {
            Ok(p) => p,
            Err(err) => {
                return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
            }
        };

    let mut payload: PaymentsRequest = stripe_payload.into();
    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(payment_id));
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
        types::StripePaymentIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            let connector = req.connector;
            payments::payments_core::<Authorize, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentConfirm,
                req,
                auth_flow,
                connector,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}

#[post("/{payment_id}/capture")]
pub async fn payment_intents_capture(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let stripe_payload: PaymentsCaptureRequest = match qs_config.deserialize_bytes(&form_payload) {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
        }
    };

    let capture_payload = PaymentsCaptureRequest {
        payment_id: Some(path.into_inner()),
        ..stripe_payload
    };

    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        capture_payload,
        |state, merchant_account, payload| {
            payments::payments_core::<Capture, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCapture,
                payload,
                api::AuthFlow::Merchant,
                None,
                payments::CallConnectorAction::Trigger,
            )
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
#[post("/{payment_id}/cancel")]
pub async fn payment_intents_cancel(
    state: web::Data<AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let payment_id = path.into_inner();
    let stripe_payload: types::StripePaymentCancelRequest =
        match qs_config.deserialize_bytes(&form_payload) {
            Ok(p) => p,
            Err(err) => {
                return api::log_and_return_error_response(report!(stripe::ErrorCode::from(err)))
            }
        };

    let mut payload: PaymentsCancelRequest = stripe_payload.into();
    payload.payment_id = payment_id;

    let auth_type = match api::get_auth_type(&req) {
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
        types::StripePaymentIntentResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<Void, api_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCancel,
                req,
                auth_flow,
                None,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all)]
#[get("/list")]
pub async fn payment_intent_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<types::StripePaymentListConstraints>,
) -> HttpResponse {
    let payload = match PaymentListConstraints::try_from(payload.into_inner()) {
        Ok(p) => p,
        Err(err) => return api::log_and_return_error_response(err),
    };
    wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentListResponse,
        stripe::ErrorCode,
    >(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::list_payments(&*state.store, merchant_account, req)
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}
