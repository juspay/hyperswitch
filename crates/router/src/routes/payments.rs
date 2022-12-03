use actix_web::{
    body::{BoxBody, MessageBody},
    web, HttpRequest, HttpResponse, Responder,
};
use error_stack::report;
use router_env::{
    tracing::{self, instrument},
    Flow,
};

use super::app::AppState;
use crate::{
    core::{errors::http_not_implemented, payments},
    services::api,
    types::{
        api::{
            payments::{
                PaymentIdType, PaymentListConstraints, PaymentsCancelRequest,
                PaymentsCaptureRequest, PaymentsRequest, PaymentsRetrieveRequest,
            },
            Authorize, Capture, PSync, PaymentRetrieveBody, PaymentsStartRequest, Void,
        },
        storage::enums::CaptureMethod,
    }, // FIXME imports
};

#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreate))]
// #[post("")]
pub async fn payments_create(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<PaymentsRequest>,
) -> HttpResponse {
    let payload = json_payload.into_inner();

    if let Some(CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<Authorize, _, _, _>(
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

#[instrument(skip(state), fields(flow = ?Flow::PaymentsStart))]
pub async fn payments_start(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
) -> HttpResponse {
    let (payment_id, merchant_id, attempt_id) = path.into_inner();
    let payload = PaymentsStartRequest {
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        txn_id: attempt_id.clone(),
    };
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<Authorize, _, _, _>(
                state,
                merchant_account,
                payments::operations::PaymentStart,
                req,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
            )
        },
        api::MerchantAuthentication::MerchantId(&merchant_id),
    )
    .await
}

#[instrument(skip(state), fields(flow = ?Flow::PaymentsRetrieve))]
// #[get("/{payment_id}")]
pub async fn payments_retrieve(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    json_payload: web::Query<PaymentRetrieveBody>,
) -> HttpResponse {
    let payload = PaymentsRetrieveRequest {
        resource_id: PaymentIdType::PaymentIntentId(path.to_string()),
        merchant_id: json_payload.merchant_id.clone(),
        force_sync: json_payload.force_sync.unwrap_or(false),
        param: None,
        connector: None,
    };
    let auth_type = match api::get_auth_type(&req) {
        Ok(auth_type) => auth_type,
        Err(err) => return api::log_and_return_error_response(report!(err)),
    };
    let _auth_flow = api::get_auth_flow(&auth_type);

    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<PSync, _, _, _>(
                state,
                merchant_account,
                payments::PaymentStatus,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        auth_type,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsUpdate))]
// #[post("/{payment_id}")]
pub async fn payments_update(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<PaymentsRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let mut payload = json_payload.into_inner();

    if let Some(CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    let payment_id = path.into_inner();

    payload.payment_id = Some(PaymentIdType::PaymentIntentId(payment_id));

    let auth_type;
    (payload, auth_type) = match api::get_auth_type_and_check_client_secret(&req, payload) {
        Ok(values) => values,
        Err(err) => return api::log_and_return_error_response(err),
    };
    let auth_flow = api::get_auth_flow(&auth_type);

    // return http_not_implemented();
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<Authorize, _, _, _>(
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

#[instrument(skip_all, fields(flow = ?Flow::PaymentsConfirm))]
// #[post("/{payment_id}/confirm")]
pub async fn payments_confirm(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<PaymentsRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let mut payload = json_payload.into_inner();

    if let Some(CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    let payment_id = path.into_inner();
    payload.payment_id = Some(PaymentIdType::PaymentIntentId(payment_id));
    payload.confirm = Some(true);

    let auth_type;
    (payload, auth_type) = match api::get_auth_type_and_check_client_secret(&req, payload) {
        Ok(values) => values,
        Err(err) => return api::log_and_return_error_response(err),
    };

    let auth_flow = api::get_auth_flow(&auth_type);
    api::server_wrap(
        &state,
        &req,
        payload,
        |state, merchant_account, req| {
            payments::payments_core::<Authorize, _, _, _>(
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

#[instrument(skip_all, fields(flow = ?Flow::PaymentsCapture))]
// #[post("/{payment_id}/capture")]
pub(crate) async fn payments_capture(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<PaymentsCaptureRequest>,
    path: web::Path<String>,
) -> HttpResponse {
    let capture_payload = PaymentsCaptureRequest {
        payment_id: Some(path.into_inner()),
        ..json_payload.into_inner()
    };

    api::server_wrap(
        &state,
        &req,
        capture_payload,
        |state, merchant_account, payload| {
            payments::payments_core::<Capture, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCapture,
                payload,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all)]
pub async fn payments_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String, String)>,
) -> HttpResponse {
    let (payment_id, merchant_id, connector) = path.into_inner();
    let param_string = req.query_string();

    let payload = PaymentsRetrieveRequest {
        resource_id: PaymentIdType::PaymentIntentId(payment_id),
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
            payments::handle_payments_redirect_response::<PSync>(state, merchant_account, req)
        },
        api::MerchantAuthentication::MerchantId(&merchant_id),
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsCancel))]
// #[post("/{payment_id}/cancel")]
pub async fn payments_cancel(
    state: web::Data<AppState>,
    req: HttpRequest,
    json_payload: web::Json<PaymentsCancelRequest>,
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
            payments::payments_core::<Void, _, _, _>(
                state,
                merchant_account,
                payments::PaymentCancel,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
            )
        },
        api::MerchantAuthentication::ApiKey,
    )
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
// #[get("/list")]
pub async fn payments_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: web::Query<PaymentListConstraints>,
) -> HttpResponse {
    let payload = payload.into_inner();
    api::server_wrap(
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

fn _http_response<T: MessageBody + 'static>(response: T) -> HttpResponse<BoxBody> {
    HttpResponse::Ok()
        .content_type("application/json")
        .append_header(("Via", "Juspay_router"))
        .body(response)
}
