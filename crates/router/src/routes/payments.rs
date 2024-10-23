use crate::{
    core::api_locking::{self, GetLockingInput},
    services::authorization::permissions::Permission,
};
pub mod helpers;

use actix_web::{web, Responder};
use api_models::payments::HeaderPayload;
use common_enums::EntityType;
use error_stack::report;
use masking::PeekInterface;
use router_env::{env, instrument, logger, tracing, types, Flow};

use super::app::ReqState;
use crate::{
    self as app,
    core::{
        errors::{self, http_not_implemented},
        payments::{self, PaymentRedirectFlow},
    },
    routes::lock_utils,
    services::{api, authentication as auth},
    types::{
        api::{
            self as api_types, enums as api_enums,
            payments::{self as payment_types, PaymentIdTypeExt},
        },
        domain,
        transformers::ForeignTryFrom,
    },
};

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreate, payment_id))]
pub async fn payments_create(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRequest>,
) -> impl Responder {
    let flow = Flow::PaymentsCreate;
    let mut payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    if let Err(err) = get_or_generate_payment_id(&mut payload) {
        return api::log_and_return_error_response(err);
    }

    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => {
            return api::log_and_return_error_response(err);
        }
    };

    tracing::Span::current().record(
        "payment_id",
        payload
            .payment_id
            .as_ref()
            .map(|payment_id_type| payment_id_type.get_payment_intent_id())
            .transpose()
            .unwrap_or_default()
            .as_ref()
            .map(|id| id.get_string_repr())
            .unwrap_or_default(),
    );

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            authorize_verify_select::<_>(
                payments::PaymentCreate,
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                header_payload.clone(),
                req,
                api::AuthFlow::Merchant,
            )
        },
        match env::which() {
            env::Env::Production => &auth::HeaderAuth(auth::ApiKeyAuth),
            _ => auth::auth_type(
                &auth::HeaderAuth(auth::ApiKeyAuth),
                &auth::JWTAuth {
                    permission: Permission::PaymentWrite,
                    minimum_entity_level: EntityType::Profile,
                },
                req.headers(),
            ),
        },
        locking_action,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreateIntent, payment_id))]
pub async fn payments_create_intent(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsCreateIntentRequest>,
) -> impl Responder {
    use hyperswitch_domain_models::payments::PaymentIntentData;

    let flow = Flow::PaymentsCreateIntent;
    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => {
            return api::log_and_return_error_response(err);
        }
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        json_payload.into_inner(),
        |state, auth: auth::AuthenticationDataV2, req, req_state| {
            payments::payments_intent_core::<
                api_types::CreateIntent,
                payment_types::PaymentsCreateIntentResponse,
                _,
                _,
                PaymentIntentData<api_types::CreateIntent>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile,
                auth.key_store,
                payments::operations::PaymentCreateIntent,
                req,
                api::AuthFlow::Client,
                header_payload.clone(),
            )
        },
        match env::which() {
            env::Env::Production => &auth::HeaderAuth(auth::ApiKeyAuth),
            _ => auth::auth_type(
                &auth::HeaderAuth(auth::ApiKeyAuth),
                &auth::JWTAuth {
                    permission: Permission::PaymentWrite,
                    minimum_entity_level: EntityType::Profile,
                },
                req.headers(),
            ),
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip(state, req), fields(flow = ?Flow::PaymentsStart, payment_id))]
pub async fn payments_start(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(
        common_utils::id_type::PaymentId,
        common_utils::id_type::MerchantId,
        String,
    )>,
) -> impl Responder {
    let flow = Flow::PaymentsStart;
    let (payment_id, merchant_id, attempt_id) = path.into_inner();
    let payload = payment_types::PaymentsStartRequest {
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        attempt_id: attempt_id.clone(),
    };

    let locking_action = payload.get_locking_input(flow.clone());
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            payments::payments_core::<
                api_types::Authorize,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Authorize>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::operations::PaymentStart,
                req,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip(state, req), fields(flow, payment_id))]
pub async fn payments_retrieve(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<common_utils::id_type::PaymentId>,
    json_payload: web::Query<payment_types::PaymentRetrieveBody>,
) -> impl Responder {
    let flow = match json_payload.force_sync {
        Some(true) => Flow::PaymentsRetrieveForceSync,
        _ => Flow::PaymentsRetrieve,
    };
    let payment_id = path.into_inner();
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: json_payload.merchant_id.clone(),
        force_sync: json_payload.force_sync.unwrap_or(false),
        client_secret: json_payload.client_secret.clone(),
        expand_attempts: json_payload.expand_attempts,
        expand_captures: json_payload.expand_captures,
        ..Default::default()
    };
    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => {
            return api::log_and_return_error_response(err);
        }
    };

    tracing::Span::current().record("flow", flow.to_string());

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(report!(err)),
        };

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
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
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentStatus,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                header_payload.clone(),
            )
        },
        auth::auth_type(
            &*auth_type,
            &auth::JWTAuth {
                permission: Permission::PaymentRead,
                minimum_entity_level: EntityType::Profile,
            },
            req.headers(),
        ),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip(state, req), fields(flow, payment_id))]
pub async fn payments_retrieve_with_gateway_creds(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentRetrieveBodyWithCredentials>,
) -> impl Responder {
    let (auth_type, _auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    tracing::Span::current().record("payment_id", json_payload.payment_id.get_string_repr());

    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(json_payload.payment_id.clone()),
        merchant_id: json_payload.merchant_id.clone(),
        force_sync: json_payload.force_sync.unwrap_or(false),
        merchant_connector_details: json_payload.merchant_connector_details.clone(),
        ..Default::default()
    };

    let flow = match json_payload.force_sync {
        Some(true) => Flow::PaymentsRetrieveForceSync,
        _ => Flow::PaymentsRetrieve,
    };

    tracing::Span::current().record("flow", flow.to_string());

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
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
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentStatus,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsUpdate, payment_id))]
pub async fn payments_update(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsUpdate;
    let mut payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    let payment_id = path.into_inner();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    payload.payment_id = Some(payment_types::PaymentIdType::PaymentIntentId(payment_id));

    let (auth_type, auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            authorize_verify_select::<_>(
                payments::PaymentUpdate,
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                HeaderPayload::default(),
                req,
                auth_flow,
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsPostSessionTokens, payment_id))]
pub async fn payments_post_session_tokens(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsPostSessionTokensRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsPostSessionTokens;

    let payment_id = path.into_inner();
    let payload = payment_types::PaymentsPostSessionTokensRequest {
        payment_id,
        ..json_payload.into_inner()
    };
    tracing::Span::current().record("payment_id", payload.payment_id.get_string_repr());
    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => {
            return api::log_and_return_error_response(err);
        }
    };

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req, req_state| {
            payments::payments_core::<
                api_types::PostSessionTokens,
                payment_types::PaymentsPostSessionTokensResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::PostSessionTokens>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentPostSessionTokens,
                req,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
                None,
                header_payload.clone(),
            )
        },
        &auth::PublishableKeyAuth,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsConfirm, payment_id))]
pub async fn payments_confirm(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsConfirm;
    let mut payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    if let Err(err) = helpers::populate_ip_into_browser_info(&req, &mut payload) {
        return api::log_and_return_error_response(err);
    }

    let payment_id = path.into_inner();
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());
    payload.payment_id = Some(payment_types::PaymentIdType::PaymentIntentId(payment_id));
    payload.confirm = Some(true);
    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(err) => {
            return api::log_and_return_error_response(err);
        }
    };

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(e) => return api::log_and_return_error_response(e),
        };

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            authorize_verify_select::<_>(
                payments::PaymentConfirm,
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                header_payload.clone(),
                req,
                auth_flow,
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCapture, payment_id))]
pub async fn payments_capture(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsCaptureRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let payment_id = path.into_inner();
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    let flow = Flow::PaymentsCapture;
    let payload = payment_types::PaymentsCaptureRequest {
        payment_id,
        ..json_payload.into_inner()
    };

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, req_state| {
            payments::payments_core::<
                api_types::Capture,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Capture>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentCapture,
                payload,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::SessionUpdateTaxCalculation, payment_id))]
pub async fn payments_dynamic_tax_calculation(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsDynamicTaxCalculationRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::SessionUpdateTaxCalculation;
    let payment_id = path.into_inner();
    let payload = payment_types::PaymentsDynamicTaxCalculationRequest {
        payment_id,
        ..json_payload.into_inner()
    };
    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(error) => {
            logger::error!(
                ?error,
                "Failed to get headers in payments_connector_session"
            );
            HeaderPayload::default()
        }
    };
    tracing::Span::current().record("payment_id", payload.payment_id.get_string_repr());
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, req_state| {
            payments::payments_core::<
                api_types::SdkSessionUpdate,
                payment_types::PaymentsDynamicTaxCalculationResponse,
                _,
                _,
                _,
                _,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentSessionUpdate,
                payload,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
                None,
                header_payload.clone(),
            )
        },
        &auth::PublishableKeyAuth,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsSessionToken, payment_id))]
pub async fn payments_connector_session(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsSessionRequest>,
) -> impl Responder {
    "Session Response"
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsSessionToken, payment_id))]
pub async fn payments_connector_session(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsSessionRequest>,
) -> impl Responder {
    let flow = Flow::PaymentsSessionToken;
    let payload = json_payload.into_inner();

    let header_payload = match HeaderPayload::foreign_try_from(req.headers()) {
        Ok(headers) => headers,
        Err(error) => {
            logger::error!(
                ?error,
                "Failed to get headers in payments_connector_session"
            );
            HeaderPayload::default()
        }
    };

    tracing::Span::current().record("payment_id", payload.payment_id.get_string_repr());

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, req_state| {
            payments::payments_core::<
                api_types::Session,
                payment_types::PaymentsSessionResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Session>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentSession,
                payload,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
                None,
                header_payload.clone(),
            )
        },
        &auth::HeaderAuth(auth::PublishableKeyAuth),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsRedirect, payment_id))]
pub async fn payments_redirect_response(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: Option<web::Form<serde_json::Value>>,
    path: web::Path<(
        common_utils::id_type::PaymentId,
        common_utils::id_type::MerchantId,
        String,
    )>,
) -> impl Responder {
    let flow = Flow::PaymentsRedirect;
    let (payment_id, merchant_id, connector) = path.into_inner();
    let param_string = req.query_string();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    let payload = payments::PaymentsRedirectResponseData {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: Some(merchant_id.clone()),
        force_sync: true,
        json_payload: json_payload.map(|payload| payload.0),
        param: Some(param_string.to_string()),
        connector: Some(connector),
        creds_identifier: None,
    };
    let locking_action = payload.get_locking_input(flow.clone());
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            <payments::PaymentRedirectSync as PaymentRedirectFlow>::handle_payments_redirect_response(
                &payments::PaymentRedirectSync {},
                state,
                req_state,
                auth.merchant_account,
                auth.key_store,
                req,

            )
        },
        &auth::MerchantIdAuth(merchant_id),
        locking_action,
    )
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsRedirect, payment_id))]
pub async fn payments_redirect_response_with_creds_identifier(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(
        common_utils::id_type::PaymentId,
        common_utils::id_type::MerchantId,
        String,
        String,
    )>,
) -> impl Responder {
    let (payment_id, merchant_id, connector, creds_identifier) = path.into_inner();
    let param_string = req.query_string();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    let payload = payments::PaymentsRedirectResponseData {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: Some(merchant_id.clone()),
        force_sync: true,
        json_payload: None,
        param: Some(param_string.to_string()),
        connector: Some(connector),
        creds_identifier: Some(creds_identifier),
    };
    let flow = Flow::PaymentsRedirect;
    let locking_action = payload.get_locking_input(flow.clone());
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
           <payments::PaymentRedirectSync as PaymentRedirectFlow>::handle_payments_redirect_response(
                &payments::PaymentRedirectSync {},
                state,
                req_state,
                auth.merchant_account,
                auth.key_store,
                req,

            )
        },
        &auth::MerchantIdAuth(merchant_id),
        locking_action,
    )
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow =? Flow::PaymentsRedirect, payment_id))]
pub async fn payments_complete_authorize_redirect(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: Option<web::Form<serde_json::Value>>,
    path: web::Path<(
        common_utils::id_type::PaymentId,
        common_utils::id_type::MerchantId,
        String,
    )>,
) -> impl Responder {
    let flow = Flow::PaymentsRedirect;
    let (payment_id, merchant_id, connector) = path.into_inner();
    let param_string = req.query_string();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    let payload = payments::PaymentsRedirectResponseData {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: Some(merchant_id.clone()),
        param: Some(param_string.to_string()),
        json_payload: json_payload.map(|s| s.0),
        force_sync: false,
        connector: Some(connector),
        creds_identifier: None,
    };
    let locking_action = payload.get_locking_input(flow.clone());
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {

            <payments::PaymentRedirectCompleteAuthorize as PaymentRedirectFlow>::handle_payments_redirect_response(
                &payments::PaymentRedirectCompleteAuthorize {},
                state,
                req_state,
                auth.merchant_account,
                auth.key_store,
                req,

            )
        },
        &auth::MerchantIdAuth(merchant_id),
        locking_action,
    )
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow =? Flow::PaymentsCompleteAuthorize, payment_id))]
pub async fn payments_complete_authorize(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsCompleteAuthorizeRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsCompleteAuthorize;
    let mut payload = json_payload.into_inner();

    let payment_id = path.into_inner();
    payload.payment_id.clone_from(&payment_id);

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    let payment_confirm_req = payment_types::PaymentsRequest {
        payment_id: Some(payment_types::PaymentIdType::PaymentIntentId(
            payment_id.clone(),
        )),
        shipping: payload.shipping.clone(),
        client_secret: Some(payload.client_secret.peek().clone()),
        ..Default::default()
    };

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payment_confirm_req) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(report!(err)),
        };

    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, _req, req_state| {
            payments::payments_core::<
                api_types::CompleteAuthorize,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::CompleteAuthorize>,
            >(
                state.clone(),
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::operations::payment_complete_authorize::CompleteAuthorize,
                payment_confirm_req.clone(),
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCancel, payment_id))]
pub async fn payments_cancel(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsCancelRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsCancel;
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    payload.payment_id = payment_id;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            payments::payments_core::<
                api_types::Void,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Void>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentCancel,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        locking_action,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn payments_list(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<payment_types::PaymentListConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::list_payments(state, auth.merchant_account, None, auth.key_store, req)
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::PaymentRead,
                minimum_entity_level: EntityType::Merchant,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn profile_payments_list(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<payment_types::PaymentListConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::list_payments(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                auth.key_store,
                req,
            )
        },
        auth::auth_type(
            &auth::HeaderAuth(auth::ApiKeyAuth),
            &auth::JWTAuth {
                permission: Permission::PaymentRead,
                minimum_entity_level: EntityType::Profile,
            },
            req.headers(),
        ),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn payments_list_by_filter(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<payment_types::PaymentListFilterConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::apply_filters_on_payments(
                state,
                auth.merchant_account,
                None,
                auth.key_store,
                req,
            )
        },
        &auth::JWTAuth {
            permission: Permission::PaymentRead,
            minimum_entity_level: EntityType::Merchant,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn profile_payments_list_by_filter(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<payment_types::PaymentListFilterConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::apply_filters_on_payments(
                state,
                auth.merchant_account.clone(),
                auth.profile_id.map(|profile_id| vec![profile_id]),
                auth.key_store,
                req,
            )
        },
        &auth::JWTAuth {
            permission: Permission::PaymentRead,
            minimum_entity_level: EntityType::Profile,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_filters_for_payments(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<common_utils::types::TimeRange>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::get_filters_for_payments(state, auth.merchant_account, auth.key_store, req)
        },
        &auth::JWTAuth {
            permission: Permission::PaymentRead,
            minimum_entity_level: EntityType::Merchant,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsFilters))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_payment_filters(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let flow = Flow::PaymentsFilters;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            payments::get_payment_filters(state, auth.merchant_account, None)
        },
        &auth::JWTAuth {
            permission: Permission::PaymentRead,
            minimum_entity_level: EntityType::Merchant,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsFilters))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_payment_filters_profile(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let flow = Flow::PaymentsFilters;
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        (),
        |state, auth: auth::AuthenticationData, _, _| {
            payments::get_payment_filters(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
            )
        },
        &auth::JWTAuth {
            permission: Permission::PaymentRead,
            minimum_entity_level: EntityType::Profile,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsAggregate))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_payments_aggregates(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<common_utils::types::TimeRange>,
) -> impl Responder {
    let flow = Flow::PaymentsAggregate;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::get_aggregates_for_payments(state, auth.merchant_account, None, req)
        },
        &auth::JWTAuth {
            permission: Permission::PaymentRead,
            minimum_entity_level: EntityType::Merchant,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(all(feature = "oltp", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsApprove, payment_id))]
pub async fn payments_approve(
    state: web::Data<app::AppState>,
    http_req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsApproveRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    payload.payment_id = payment_id;
    let flow = Flow::PaymentsApprove;
    let fpayload = FPaymentsApproveRequest(&payload);
    let locking_action = fpayload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.clone(),
        |state, auth: auth::AuthenticationData, req, req_state| {
            payments::payments_core::<
                api_types::Capture,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Capture>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentApprove,
                payment_types::PaymentsCaptureRequest {
                    payment_id: req.payment_id,
                    ..Default::default()
                },
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        match env::which() {
            env::Env::Production => &auth::HeaderAuth(auth::ApiKeyAuth),
            _ => auth::auth_type(
                &auth::HeaderAuth(auth::ApiKeyAuth),
                &auth::JWTAuth {
                    permission: Permission::PaymentWrite,
                    minimum_entity_level: EntityType::Profile,
                },
                http_req.headers(),
            ),
        },
        locking_action,
    ))
    .await
}

#[cfg(all(feature = "oltp", feature = "v1"))]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsReject, payment_id))]
pub async fn payments_reject(
    state: web::Data<app::AppState>,
    http_req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRejectRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    payload.payment_id = payment_id;
    let flow = Flow::PaymentsReject;
    let fpayload = FPaymentsRejectRequest(&payload);
    let locking_action = fpayload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &http_req,
        payload.clone(),
        |state, auth: auth::AuthenticationData, req, req_state| {
            payments::payments_core::<
                api_types::Void,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::Void>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentReject,
                payment_types::PaymentsCancelRequest {
                    payment_id: req.payment_id,
                    cancellation_reason: Some("Rejected by merchant".to_string()),
                    ..Default::default()
                },
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        match env::which() {
            env::Env::Production => &auth::HeaderAuth(auth::ApiKeyAuth),
            _ => auth::auth_type(
                &auth::HeaderAuth(auth::ApiKeyAuth),
                &auth::JWTAuth {
                    permission: Permission::PaymentWrite,
                    minimum_entity_level: EntityType::Profile,
                },
                http_req.headers(),
            ),
        },
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[allow(clippy::too_many_arguments)]
async fn authorize_verify_select<Op>(
    operation: Op,
    state: app::SessionState,
    req_state: ReqState,
    merchant_account: domain::MerchantAccount,
    profile_id: Option<common_utils::id_type::ProfileId>,
    key_store: domain::MerchantKeyStore,
    header_payload: HeaderPayload,
    req: api_models::payments::PaymentsRequest,
    auth_flow: api::AuthFlow,
) -> errors::RouterResponse<api_models::payments::PaymentsResponse>
where
    Op: Sync
        + Clone
        + std::fmt::Debug
        + payments::operations::Operation<
            api_types::Authorize,
            api_models::payments::PaymentsRequest,
            Data = payments::PaymentData<api_types::Authorize>,
        > + payments::operations::Operation<
            api_types::SetupMandate,
            api_models::payments::PaymentsRequest,
            Data = payments::PaymentData<api_types::SetupMandate>,
        >,
{
    // TODO: Change for making it possible for the flow to be inferred internally or through validation layer
    // This is a temporary fix.
    // After analyzing the code structure,
    // the operation are flow agnostic, and the flow is only required in the post_update_tracker
    // Thus the flow can be generated just before calling the connector instead of explicitly passing it here.

    let is_recurring_details_type_nti_and_card_details = req
        .recurring_details
        .clone()
        .map(|recurring_details| {
            recurring_details.is_network_transaction_id_and_card_details_flow()
        })
        .unwrap_or(false);
    if is_recurring_details_type_nti_and_card_details {
        // no list of eligible connectors will be passed in the confirm call
        logger::debug!("Authorize call for NTI and Card Details flow");
        payments::proxy_for_payments_core::<
            api_types::Authorize,
            payment_types::PaymentsResponse,
            _,
            _,
            _,
            payments::PaymentData<api_types::Authorize>,
        >(
            state,
            req_state,
            merchant_account,
            profile_id,
            key_store,
            operation,
            req,
            auth_flow,
            payments::CallConnectorAction::Trigger,
            header_payload,
        )
        .await
    } else {
        let eligible_connectors = req.connector.clone();
        match req.payment_type.unwrap_or_default() {
            api_models::enums::PaymentType::Normal
            | api_models::enums::PaymentType::RecurringMandate
            | api_models::enums::PaymentType::NewMandate => {
                payments::payments_core::<
                    api_types::Authorize,
                    payment_types::PaymentsResponse,
                    _,
                    _,
                    _,
                    payments::PaymentData<api_types::Authorize>,
                >(
                    state,
                    req_state,
                    merchant_account,
                    profile_id,
                    key_store,
                    operation,
                    req,
                    auth_flow,
                    payments::CallConnectorAction::Trigger,
                    eligible_connectors,
                    header_payload,
                )
                .await
            }
            api_models::enums::PaymentType::SetupMandate => {
                payments::payments_core::<
                    api_types::SetupMandate,
                    payment_types::PaymentsResponse,
                    _,
                    _,
                    _,
                    payments::PaymentData<api_types::SetupMandate>,
                >(
                    state,
                    req_state,
                    merchant_account,
                    profile_id,
                    key_store,
                    operation,
                    req,
                    auth_flow,
                    payments::CallConnectorAction::Trigger,
                    eligible_connectors,
                    header_payload,
                )
                .await
            }
        }
    }
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsIncrementalAuthorization, payment_id))]
pub async fn payments_incremental_authorization(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsIncrementalAuthorizationRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsIncrementalAuthorization;
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    payload.payment_id = payment_id;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            payments::payments_core::<
                api_types::IncrementalAuthorization,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                payments::PaymentData<api_types::IncrementalAuthorization>,
            >(
                state,
                req_state,
                auth.merchant_account,
                auth.profile_id,
                auth.key_store,
                payments::PaymentIncrementalAuthorization,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsExternalAuthentication, payment_id))]
pub async fn payments_external_authentication(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsExternalAuthenticationRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsExternalAuthentication;
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    payload.payment_id = payment_id;
    let locking_action = payload.get_locking_input(flow.clone());
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::payment_external_authentication(
                state,
                auth.merchant_account,
                auth.key_store,
                req,
            )
        },
        &auth::HeaderAuth(auth::PublishableKeyAuth),
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsAuthorize, payment_id))]
pub async fn post_3ds_payments_authorize(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: Option<web::Form<serde_json::Value>>,
    path: web::Path<(
        common_utils::id_type::PaymentId,
        common_utils::id_type::MerchantId,
        String,
    )>,
) -> impl Responder {
    let flow = Flow::PaymentsAuthorize;

    let (payment_id, merchant_id, connector) = path.into_inner();
    tracing::Span::current().record("payment_id", payment_id.get_string_repr());
    let param_string = req.query_string();
    let payload = payments::PaymentsRedirectResponseData {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: Some(merchant_id.clone()),
        force_sync: true,
        json_payload: json_payload.map(|payload| payload.0),
        param: Some(param_string.to_string()),
        connector: Some(connector),
        creds_identifier: None,
    };

    let locking_action = payload.get_locking_input(flow.clone());

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, req_state| {
            <payments::PaymentAuthenticateCompleteAuthorize as PaymentRedirectFlow>::handle_payments_redirect_response(
                &payments::PaymentAuthenticateCompleteAuthorize {},
                state,
                req_state,
                auth.merchant_account,
                auth.key_store,
                req,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        locking_action,
    ))
    .await
}

#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn payments_manual_update(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsManualUpdateRequest>,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::PaymentsManualUpdate;
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();

    let locking_action = payload.get_locking_input(flow.clone());

    tracing::Span::current().record("payment_id", payment_id.get_string_repr());

    payload.payment_id = payment_id;

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, _auth, req, _req_state| payments::payments_manual_update(state, req),
        &auth::AdminApiAuthWithMerchantIdFromHeader,
        locking_action,
    ))
    .await
}

#[cfg(feature = "v1")]
/// Retrieve endpoint for merchant to fetch the encrypted customer payment method data
#[instrument(skip_all, fields(flow = ?Flow::GetExtendedCardInfo, payment_id))]
pub async fn retrieve_extended_card_info(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<common_utils::id_type::PaymentId>,
) -> impl Responder {
    let flow = Flow::GetExtendedCardInfo;
    let payment_id = path.into_inner();

    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payment_id,
        |state, auth: auth::AuthenticationData, payment_id, _| {
            payments::get_extended_card_info(
                state,
                auth.merchant_account.get_id().to_owned(),
                payment_id,
            )
        },
        &auth::HeaderAuth(auth::ApiKeyAuth),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
pub fn get_or_generate_payment_id(
    payload: &mut payment_types::PaymentsRequest,
) -> errors::RouterResult<()> {
    let given_payment_id = payload
        .payment_id
        .clone()
        .map(|payment_id| {
            payment_id
                .get_payment_intent_id()
                .map_err(|err| err.change_context(errors::ApiErrorResponse::PaymentNotFound))
        })
        .transpose()?;

    let payment_id = given_payment_id.unwrap_or(common_utils::id_type::PaymentId::default());

    payload.payment_id = Some(api_models::payments::PaymentIdType::PaymentIntentId(
        payment_id,
    ));

    Ok(())
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        match self.payment_id {
            Some(payment_types::PaymentIdType::PaymentIntentId(ref id)) => {
                api_locking::LockAction::Hold {
                    input: api_locking::LockingInput {
                        unique_locking_key: id.get_string_repr().to_owned(),
                        api_identifier: lock_utils::ApiIdentifier::from(flow),
                        override_lock_retries: None,
                    },
                }
            }
            _ => api_locking::LockAction::NotApplicable,
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsStartRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsRetrieveRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        match self.resource_id {
            payment_types::PaymentIdType::PaymentIntentId(ref id) if self.force_sync => {
                api_locking::LockAction::Hold {
                    input: api_locking::LockingInput {
                        unique_locking_key: id.get_string_repr().to_owned(),
                        api_identifier: lock_utils::ApiIdentifier::from(flow),
                        override_lock_retries: None,
                    },
                }
            }
            _ => api_locking::LockAction::NotApplicable,
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsSessionRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsDynamicTaxCalculationRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsPostSessionTokensRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payments::PaymentsRedirectResponseData {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        match self.resource_id {
            payment_types::PaymentIdType::PaymentIntentId(ref id) => {
                api_locking::LockAction::Hold {
                    input: api_locking::LockingInput {
                        unique_locking_key: id.get_string_repr().to_owned(),
                        api_identifier: lock_utils::ApiIdentifier::from(flow),
                        override_lock_retries: None,
                    },
                }
            }
            _ => api_locking::LockAction::NotApplicable,
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsCompleteAuthorizeRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsCancelRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsCaptureRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "oltp")]
struct FPaymentsApproveRequest<'a>(&'a payment_types::PaymentsApproveRequest);

#[cfg(feature = "oltp")]
impl<'a> GetLockingInput for FPaymentsApproveRequest<'a> {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.0.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "oltp")]
struct FPaymentsRejectRequest<'a>(&'a payment_types::PaymentsRejectRequest);

#[cfg(feature = "oltp")]
impl<'a> GetLockingInput for FPaymentsRejectRequest<'a> {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.0.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsIncrementalAuthorizationRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsExternalAuthenticationRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[cfg(feature = "v1")]
impl GetLockingInput for payment_types::PaymentsManualUpdateRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.get_string_repr().to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsAggregate))]
#[cfg(all(feature = "olap", feature = "v1"))]
pub async fn get_payments_aggregates_profile(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<common_utils::types::TimeRange>,
) -> impl Responder {
    let flow = Flow::PaymentsAggregate;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, req, _| {
            payments::get_aggregates_for_payments(
                state,
                auth.merchant_account,
                auth.profile_id.map(|profile_id| vec![profile_id]),
                req,
            )
        },
        &auth::JWTAuth {
            permission: Permission::PaymentRead,
            minimum_entity_level: EntityType::Profile,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
