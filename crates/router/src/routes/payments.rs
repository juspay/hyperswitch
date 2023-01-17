use actix_web::{web, Responder};
use error_stack::report;
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{errors::http_not_implemented, payments},
    services::{api, authentication as auth},
    types::api::{self as api_types, enums as api_enums, payments as payment_types},
};

/// Payments - Create
///
/// To create a new payment, against a merchant API key
#[utoipa::path(
    post,
    path = "/payments",
    request_body=PaymentsRequest,
    responses(
        (status = 200, description = "Payment created", body = PaymentsResponse),
        (status = 400, description = "Missing Mandatory fields")
    )
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreate))]
// #[post("")]
pub async fn payments_create(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRequest>,
) -> impl Responder {
    let payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            authorize_verify_select(
                payments::PaymentCreate,
                state,
                merchant_account,
                req,
                api::AuthFlow::Merchant,
            )
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip(state), fields(flow = ?Flow::PaymentsStart))]
pub async fn payments_start(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let (payment_id, merchant_id, attempt_id) = path.into_inner();
    let payload = payment_types::PaymentsStartRequest {
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        txn_id: attempt_id.clone(),
    };
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<api_types::Authorize, payment_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::operations::PaymentStart,
                req,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
    )
    .await
}

#[instrument(skip(state), fields(flow = ?Flow::PaymentsRetrieve))]
// #[get("/{payment_id}")]
pub async fn payments_retrieve(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    json_payload: web::Query<payment_types::PaymentRetrieveBody>,
) -> impl Responder {
    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(path.to_string()),
        merchant_id: json_payload.merchant_id.clone(),
        force_sync: json_payload.force_sync.unwrap_or(false),
        param: None,
        connector: None,
    };
    let (auth_type, _auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<api_types::PSync, payment_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentStatus,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        &*auth_type,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsUpdate))]
// #[post("/{payment_id}")]
pub async fn payments_update(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRequest>,
    path: web::Path<String>,
) -> impl Responder {
    let mut payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    let payment_id = path.into_inner();

    payload.payment_id = Some(payment_types::PaymentIdType::PaymentIntentId(payment_id));

    let (auth_type, auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            authorize_verify_select(
                payments::PaymentUpdate,
                state,
                merchant_account,
                req,
                auth_flow,
            )
        },
        &*auth_type,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsConfirm))]
// #[post("/{payment_id}/confirm")]
pub async fn payments_confirm(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRequest>,
    path: web::Path<String>,
) -> impl Responder {
    let mut payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    let payment_id = path.into_inner();
    payload.payment_id = Some(payment_types::PaymentIdType::PaymentIntentId(payment_id));
    payload.confirm = Some(true);

    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(e) => return api::log_and_return_error_response(e),
        };

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            authorize_verify_select(
                payments::PaymentConfirm,
                state,
                merchant_account,
                req,
                auth_flow,
            )
        },
        &*auth_type,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsCapture))]
// #[post("/{payment_id}/capture")]
pub async fn payments_capture(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsCaptureRequest>,
    path: web::Path<String>,
) -> impl Responder {
    let capture_payload = payment_types::PaymentsCaptureRequest {
        payment_id: Some(path.into_inner()),
        ..json_payload.into_inner()
    };

    api::server_wrap(
        &state,
        &req,
        capture_payload,
        |state, merchant_account, payload| {
            payments::payments_core::<api_types::Capture, payment_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCapture,
                payload,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsSessionToken))]
pub async fn payments_connector_session(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsSessionRequest>,
) -> impl Responder {
    let sessions_payload = json_payload.into_inner();

    api::server_wrap(
        &state,
        &req,
        sessions_payload,
        |state, merchant_account, payload| {
            payments::payments_core::<
                api_types::Session,
                payment_types::PaymentsSessionResponse,
                _,
                _,
                _,
            >(
                state,
                merchant_account,
                payments::PaymentSession,
                payload,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
            )
        },
        &auth::PublishableKeyAuth,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payments_response(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let (payment_id, merchant_id, connector) = path.into_inner();
    let param_string = req.query_string();

    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: Some(merchant_id.clone()),
        force_sync: true,
        param: Some(param_string.to_string()),
        connector: Some(connector),
    };
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::handle_payments_redirect_response::<api_types::PSync>(
                state,
                merchant_account,
                req,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsCancel))]
// #[post("/{payment_id}/cancel")]
pub async fn payments_cancel(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsCancelRequest>,
    path: web::Path<String>,
) -> impl Responder {
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();
    payload.payment_id = payment_id;

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<api_types::Void, payment_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCancel,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        &auth::ApiKeyAuth,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(feature = "olap")]
// #[get("/list")]
pub async fn payments_list(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<payment_types::PaymentListConstraints>,
) -> impl Responder {
    let payload = payload.into_inner();
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::list_payments(&*state.store, merchant_account, req)
        },
        *auth::jwt_auth_or(&auth::ApiKeyAuth, req.headers()),
    )
    .await
}

async fn authorize_verify_select<Op>(
    operation: Op,
    state: &app::AppState,
    merchant_account: storage_models::merchant_account::MerchantAccount,
    req: api_models::payments::PaymentsRequest,
    auth_flow: api::AuthFlow,
) -> app::core::errors::RouterResponse<api_models::payments::PaymentsResponse>
where
    Op: Sync
        + Clone
        + std::fmt::Debug
        + payments::operations::Operation<api_types::Authorize, api_models::payments::PaymentsRequest>
        + payments::operations::Operation<api_types::Verify, api_models::payments::PaymentsRequest>,
{
    // TODO: Change for making it possible for the flow to be inferred internally or through validation layer
    // This is a temporary fix.
    // After analyzing the code structure,
    // the operation are flow agnostic, and the flow is only required in the post_update_tracker
    // Thus the flow can be generated just before calling the connector instead of explicitly passing it here.

    match req.amount.as_ref() {
        Some(api_types::Amount::Value(_)) | None => payments::payments_core::<
            api_types::Authorize,
            payment_types::PaymentsResponse,
            _,
            _,
            _,
        >(
            state,
            merchant_account,
            operation,
            req,
            auth_flow,
            payments::CallConnectorAction::Trigger,
        )
        .await,

        Some(api_types::Amount::Zero) => {
            payments::payments_core::<api_types::Verify, payment_types::PaymentsResponse, _, _, _>(
                state,
                merchant_account,
                operation,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
            )
            .await
        }
    }
}
