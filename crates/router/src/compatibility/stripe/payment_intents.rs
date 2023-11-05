pub mod types;
use actix_web::{web, HttpRequest, HttpResponse};
use api_models::payments as payment_types;
use error_stack::report;
use router_env::{instrument, tracing, Flow};

use crate::{
    compatibility::{stripe::errors, wrap},
    core::{api_locking::GetLockingInput, payment_methods::Oss, payments},
    routes,
    services::{api, authentication as auth},
    types::api as api_types,
};

#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreate))]
pub async fn payment_intents_create(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let payload: types::StripePaymentIntentRequest = match qs_config
        .deserialize_bytes(&form_payload)
        .map_err(|err| report!(errors::StripeErrorCode::from(err)))
    {
        Ok(p) => p,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let create_payment_req: payment_types::PaymentsRequest = match payload.try_into() {
        Ok(req) => req,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let flow = Flow::PaymentsCreate;
    let locking_action = create_payment_req.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        create_payment_req,
        |state, auth, req| {
            let eligible_connectors = req.connector.clone();
            payments::payments_core::<api_types::Authorize, api_types::PaymentsResponse, _, _, _,Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentCreate,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                eligible_connectors,
                api_types::HeaderPayload::default(),
            )
        },
        &auth::ApiKeyAuth,
        locking_action,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsRetrieve))]
pub async fn payment_intents_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query_payload: web::Query<types::StripePaymentRetrieveBody>,
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
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
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
        locking_action,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsRetrieve))]
pub async fn payment_intents_retrieve_with_gateway_creds(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
) -> HttpResponse {
    let json_payload: payment_types::PaymentRetrieveBodyWithCredentials = match qs_config
        .deserialize_bytes(&form_payload)
        .map_err(|err| report!(errors::StripeErrorCode::from(err)))
    {
        Ok(p) => p,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(
            json_payload.payment_id.to_string(),
        ),
        merchant_id: json_payload.merchant_id.clone(),
        force_sync: json_payload.force_sync.unwrap_or(false),
        merchant_connector_details: json_payload.merchant_connector_details.clone(),
        ..Default::default()
    };
    let (auth_type, _auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    let flow = Flow::PaymentsRetrieve;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            payments::payments_core::<api_types::PSync, payment_types::PaymentsResponse, _, _, _,Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentStatus,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                    None,
                api_types::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsUpdate))]
pub async fn payment_intents_update(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let payment_id = path.into_inner();
    let stripe_payload: types::StripePaymentIntentRequest = match qs_config
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

    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(payment_id));

    let (auth_type, auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    let flow = Flow::PaymentsUpdate;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            let eligible_connectors = req.connector.clone();
            payments::payments_core::<api_types::Authorize, api_types::PaymentsResponse, _, _, _,Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentUpdate,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                eligible_connectors,
                api_types::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsConfirm))]
pub async fn payment_intents_confirm(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let payment_id = path.into_inner();
    let stripe_payload: types::StripePaymentIntentRequest = match qs_config
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

    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(payment_id));
    payload.confirm = Some(true);

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(err),
        };

    let flow = Flow::PaymentsConfirm;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            let eligible_connectors = req.connector.clone();
            payments::payments_core::<api_types::Authorize, api_types::PaymentsResponse, _, _, _,Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentConfirm,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                eligible_connectors,
                api_types::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCapture))]
pub async fn payment_intents_capture(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let stripe_payload: payment_types::PaymentsCaptureRequest = match qs_config
        .deserialize_bytes(&form_payload)
    {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let payload = payment_types::PaymentsCaptureRequest {
        payment_id: path.into_inner(),
        ..stripe_payload
    };

    let flow = Flow::PaymentsCapture;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, payload| {
            payments::payments_core::<api_types::Capture, api_types::PaymentsResponse, _, _, _,Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentCapture,
                payload,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                    None,
                api_types::HeaderPayload::default(),
            )
        },
        &auth::ApiKeyAuth,
        locking_action,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCancel))]
pub async fn payment_intents_cancel(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<String>,
) -> HttpResponse {
    let payment_id = path.into_inner();
    let stripe_payload: types::StripePaymentCancelRequest = match qs_config
        .deserialize_bytes(&form_payload)
    {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    let mut payload: payment_types::PaymentsCancelRequest = stripe_payload.into();
    payload.payment_id = payment_id;

    let (auth_type, auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    let flow = Flow::PaymentsCancel;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| {
            payments::payments_core::<api_types::Void, api_types::PaymentsResponse, _, _, _, Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentCancel,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                api_types::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(feature = "olap")]
pub async fn payment_intent_list(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    payload: web::Query<types::StripePaymentListConstraints>,
) -> HttpResponse {
    let payload = match payment_types::PaymentListConstraints::try_from(payload.into_inner()) {
        Ok(p) => p,
        Err(err) => return api::log_and_return_error_response(err),
    };
    use crate::core::api_locking;
    let flow = Flow::PaymentsList;
    Box::pin(wrap::compatibility_api_wrap::<
        _,
        _,
        _,
        _,
        _,
        _,
        types::StripePaymentIntentListResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req| payments::list_payments(state, auth.merchant_account, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
