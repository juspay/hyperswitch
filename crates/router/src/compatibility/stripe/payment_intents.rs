pub mod types;

use actix_web::{web, HttpRequest, HttpResponse};
use api_models::payments as payment_types;
#[cfg(feature = "v1")]
use error_stack::report;
#[cfg(feature = "v1")]
use router_env::Tag;
use router_env::{instrument, tracing, Flow};

use crate::{
    compatibility::{stripe::errors, wrap},
    core::payments,
    routes::{self},
    services::{api, authentication as auth},
};
#[cfg(feature = "v1")]
use crate::{
    core::api_locking::GetLockingInput, logger, routes::payments::get_or_generate_payment_id,
    types::api as api_types,
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreate, payment_id))]
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

    tracing::Span::current().record(
        "payment_id",
        payload
            .id
            .as_ref()
            .map(|payment_id| payment_id.get_string_repr())
            .unwrap_or_default(),
    );

    logger::info!(tag = ?Tag::CompatibilityLayerRequest, payload = ?payload);

    let mut create_payment_req: payment_types::PaymentsRequest = match payload.try_into() {
        Ok(req) => req,
        Err(err) => return api::log_and_return_error_response(err),
    };

    if let Err(err) = get_or_generate_payment_id(&mut create_payment_req) {
        return api::log_and_return_error_response(err);
    }
    let flow = Flow::PaymentsCreate;
    let locking_action = create_payment_req.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
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
        |state, auth: auth::AuthenticationData, req, req_state| {
            let platform = auth.into();
            let eligible_connectors = req.connector.clone();
            payments::payments_core::<
                api_types::Authorize,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Authorize>,
            >(
                state,
                req_state,
                platform,
                None,
                payments::PaymentCreate,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                eligible_connectors,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsRetrieveForceSync))]
pub async fn payment_intents_retrieve(
    state: web::Data<routes::AppState>,
    req: HttpRequest,
    path: web::Path<common_utils::id_type::PaymentId>,
    query_payload: web::Query<types::StripePaymentRetrieveBody>,
) -> HttpResponse {
    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: api_types::PaymentIdType::PaymentIntentId(path.into_inner()),
        merchant_id: None,
        force_sync: true,
        connector: None,
        param: None,
        merchant_connector_details: None,
        client_secret: query_payload.client_secret.clone(),
        expand_attempts: None,
        expand_captures: None,
        all_keys_required: None,
    };

    let api_auth = auth::ApiKeyAuth {
        is_connected_allowed: false,
        is_platform_allowed: false,
    };

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(report!(err)),
        };

    let flow = Flow::PaymentsRetrieveForceSync;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
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
        |state, auth, payload, req_state| {
            let platform = auth.into();
            payments::payments_core::<
                api_types::PSync,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::PSync>,
            >(
                state,
                req_state,
                platform,
                None,
                payments::PaymentStatus,
                payload,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                None,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow))]
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
        resource_id: payment_types::PaymentIdType::PaymentIntentId(json_payload.payment_id),
        merchant_id: json_payload.merchant_id.clone(),
        force_sync: json_payload.force_sync.unwrap_or(false),
        merchant_connector_details: json_payload.merchant_connector_details.clone(),
        ..Default::default()
    };

    let api_auth = auth::ApiKeyAuth {
        is_connected_allowed: false,
        is_platform_allowed: false,
    };

    let (auth_type, _auth_flow) = match auth::get_auth_type_and_flow(req.headers(), api_auth) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    let flow = match json_payload.force_sync {
        Some(true) => Flow::PaymentsRetrieveForceSync,
        _ => Flow::PaymentsRetrieve,
    };

    tracing::Span::current().record("flow", flow.to_string());

    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(wrap::compatibility_api_wrap::<
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
        |state, auth, req, req_state| {
            let platform = auth.into();
            payments::payments_core::<
                api_types::PSync,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::PSync>,
            >(
                state,
                req_state,
                platform,
                None,
                payments::PaymentStatus,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                None,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsUpdate))]
pub async fn payment_intents_update(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<common_utils::id_type::PaymentId>,
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

    let api_auth = auth::ApiKeyAuth {
        is_connected_allowed: false,
        is_platform_allowed: false,
    };

    let (auth_type, auth_flow) = match auth::get_auth_type_and_flow(req.headers(), api_auth) {
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
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req, req_state| {
            let platform = auth.into();
            let eligible_connectors = req.connector.clone();
            payments::payments_core::<
                api_types::Authorize,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Authorize>,
            >(
                state,
                req_state,
                platform,
                None,
                payments::PaymentUpdate,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                eligible_connectors,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsConfirm, payment_id))]
pub async fn payment_intents_confirm(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<common_utils::id_type::PaymentId>,
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

    tracing::Span::current().record(
        "payment_id",
        stripe_payload.id.as_ref().map(|id| id.get_string_repr()),
    );

    logger::info!(tag = ?Tag::CompatibilityLayerRequest, payload = ?stripe_payload);

    let mut payload: payment_types::PaymentsRequest = match stripe_payload.try_into() {
        Ok(req) => req,
        Err(err) => return api::log_and_return_error_response(err),
    };

    payload.payment_id = Some(api_types::PaymentIdType::PaymentIntentId(payment_id));
    payload.confirm = Some(true);

    let api_auth = auth::ApiKeyAuth {
        is_connected_allowed: false,
        is_platform_allowed: false,
    };

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
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
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req, req_state| {
            let platform = auth.into();
            let eligible_connectors = req.connector.clone();
            payments::payments_core::<
                api_types::Authorize,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Authorize>,
            >(
                state,
                req_state,
                platform,
                None,
                payments::PaymentConfirm,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                eligible_connectors,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCapture, payment_id))]
pub async fn payment_intents_capture(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> HttpResponse {
    let stripe_payload: payment_types::PaymentsCaptureRequest = match qs_config
        .deserialize_bytes(&form_payload)
    {
        Ok(p) => p,
        Err(err) => {
            return api::log_and_return_error_response(report!(errors::StripeErrorCode::from(err)))
        }
    };

    tracing::Span::current().record("payment_id", stripe_payload.payment_id.get_string_repr());

    logger::info!(tag = ?Tag::CompatibilityLayerRequest, payload = ?stripe_payload);

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
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, req_state| {
            let platform = auth.into();
            payments::payments_core::<
                api_types::Capture,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Capture>,
            >(
                state,
                req_state,
                platform,
                None,
                payments::PaymentCapture,
                payload,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                None,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCancel, payment_id))]
pub async fn payment_intents_cancel(
    state: web::Data<routes::AppState>,
    qs_config: web::Data<serde_qs::Config>,
    req: HttpRequest,
    form_payload: web::Bytes,
    path: web::Path<common_utils::id_type::PaymentId>,
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

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    logger::info!(tag = ?Tag::CompatibilityLayerRequest, payload = ?stripe_payload);

    let mut payload: payment_types::PaymentsCancelRequest = stripe_payload.into();
    payload.payment_id = payment_id;

    let api_auth = auth::ApiKeyAuth {
        is_connected_allowed: false,
        is_platform_allowed: false,
    };

    let (auth_type, auth_flow) = match auth::get_auth_type_and_flow(req.headers(), api_auth) {
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
        types::StripePaymentIntentResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth, req, req_state| {
            let platform = auth.into();
            payments::payments_core::<
                api_types::Void,
                api_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Void>,
            >(
                state,
                req_state,
                platform,
                None,
                payments::PaymentCancel,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                None,
                hyperswitch_domain_models::payments::HeaderPayload::default(),
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
        types::StripePaymentIntentListResponse,
        errors::StripeErrorCode,
        _,
    >(
        flow,
        state.into_inner(),
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            let platform = auth.into();
            payments::list_payments(state, platform, None, req)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
