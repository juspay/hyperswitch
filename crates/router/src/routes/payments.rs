use actix_web::{web, Responder};
use error_stack::report;
use router_env::{instrument, tracing, Flow};

use crate::{
    self as app,
    core::{
        errors::http_not_implemented,
        payments::{self, PaymentRedirectFlow},
    },
    services::{api, authentication as auth},
    types::api::{self as api_types, enums as api_enums, payments as payment_types},
};

/// Payments - Create
///
/// To process a payment you will have to create a payment, attach a payment method and confirm. Depending on the user journey you wish to achieve, you may opt to all the steps in a single request or in a sequence of API request using following APIs: (i) Payments - Update, (ii) Payments - Confirm, and (iii) Payments - Capture
#[utoipa::path(
    post,
    path = "/payments",
    request_body=PaymentsRequest,
    responses(
        (status = 200, description = "Payment created", body = PaymentsResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Create a Payment",
    security(("api_key" = []))
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
        state.get_ref(),
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

// /// Payments - Start
// ///
// /// The entry point for a payment which involves the redirection flow. This redirects the user to the authentication page
// #[utoipa::path(
//     get,
//     path = "/payments/start/{payment_id}/{merchant_id}/{attempt_id}",
//     params(
//         ("payment_id" = String, Path, description = "The identifier for payment"),
//         ("merchant_id" = String, Path, description = "The identifier for merchant"),
//         ("attempt_id" = String, Path, description = "The identifier for transaction")
//     ),
//     responses(
//         (status = 200, description = "Redirects to the authentication page"),
//         (status = 404, description = "No redirection found")
//     ),
//     tag = "Payments",
//     operation_id = "Start a Redirection Payment"
// )]
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
        attempt_id: attempt_id.clone(),
    };
    api::server_wrap(
        state.get_ref(),
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

/// Payments - Retrieve
///
/// To retrieve the properties of a Payment. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    get,
    path = "/payments/{payment_id}",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentRetrieveBody,
    responses(
        (status = 200, description = "Gets the payment with final status", body = PaymentsResponse),
        (status = 404, description = "No payment found")
    ),
    tag = "Payments",
    operation_id = "Retrieve a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
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
        state.get_ref(),
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

/// Payments - Update
///
/// To update the properties of a PaymentIntent object. This may include attaching a payment method, or attaching customer object or metadata fields after the Payment is created
#[utoipa::path(
    post,
    path = "/payments/{payment_id}",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsRequest,
    responses(
        (status = 200, description = "Payment updated", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Update a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
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
        state.get_ref(),
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

/// Payments - Confirm
///
/// This API is to confirm the payment request and forward payment to the payment processor. This API provides more granular control upon when the API is forwarded to the payment processor. Alternatively you can confirm the payment within the Payments Create API
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/confirm",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsRequest,
    responses(
        (status = 200, description = "Payment confirmed", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Confirm a Payment",
    security(("api_key" = []), ("publishable_key" = []))
)]
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
        state.get_ref(),
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

/// Payments - Capture
///
/// To capture the funds for an uncaptured payment
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/capture",
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    request_body=PaymentsCaptureRequest,
    responses(
        (status = 200, description = "Payment captured", body = PaymentsResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Capture a Payment",
    security(("api_key" = []))
)]
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
        state.get_ref(),
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

/// Payments - Session token
///
/// To create the session object or to get session token for wallets
#[utoipa::path(
    post,
    path = "/payments/session_tokens",
    request_body=PaymentsSessionRequest,
    responses(
        (status = 200, description = "Payment session object created or session token was retrieved from wallets", body = PaymentsSessionResponse),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Create Session tokens for a Payment",
    security(("publishable_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsSessionToken))]
pub async fn payments_connector_session(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsSessionRequest>,
) -> impl Responder {
    let sessions_payload = json_payload.into_inner();

    api::server_wrap(
        state.get_ref(),
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

// /// Payments - Redirect response
// ///
// /// To get the payment response for redirect flows
// #[utoipa::path(
//     post,
//     path = "/payments/{payment_id}/{merchant_id}/response/{connector}",
//     params(
//         ("payment_id" = String, Path, description = "The identifier for payment"),
//         ("merchant_id" = String, Path, description = "The identifier for merchant"),
//         ("connector" = String, Path, description = "The name of the connector")
//     ),
//     responses(
//         (status = 302, description = "Received payment redirect response"),
//         (status = 400, description = "Missing mandatory fields")
//     ),
//     tag = "Payments",
//     operation_id = "Get Redirect Response for a Payment"
// )]
#[instrument(skip_all)]
pub async fn payments_redirect_response(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let (payment_id, merchant_id, connector) = path.into_inner();
    let param_string = req.query_string();

    let payload = payments::PaymentsRedirectResponseData {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: Some(merchant_id.clone()),
        force_sync: true,
        json_payload: None,
        param: Some(param_string.to_string()),
        connector: Some(connector),
    };
    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, req| {
            payments::PaymentRedirectSync {}.handle_payments_redirect_response(
                state,
                merchant_account,
                req,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
    )
    .await
}

#[instrument(skip_all)]
pub async fn payments_complete_authorize(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Form<serde_json::Value>,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let (payment_id, merchant_id, connector) = path.into_inner();
    let param_string = req.query_string();

    let payload = payments::PaymentsRedirectResponseData {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(payment_id),
        merchant_id: Some(merchant_id.clone()),
        param: Some(param_string.to_string()),
        json_payload: Some(json_payload.0),
        force_sync: false,
        connector: Some(connector),
    };
    api::server_wrap(
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, req| {
            payments::PaymentRedirectCompleteAuthorize {}.handle_payments_redirect_response(
                state,
                merchant_account,
                req,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
    )
    .await
}

/// Payments - Cancel
///
/// A Payment could can be cancelled when it is in one of these statuses: requires_payment_method, requires_capture, requires_confirmation, requires_customer_action
#[utoipa::path(
    post,
    path = "/payments/{payment_id}/cancel",
    request_body=PaymentsCancelRequest,
    params(
        ("payment_id" = String, Path, description = "The identifier for payment")
    ),
    responses(
        (status = 200, description = "Payment canceled"),
        (status = 400, description = "Missing mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Cancel a Payment",
    security(("api_key" = []))
)]
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
        state.get_ref(),
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

/// Payments - List
///
/// To list the payments
#[utoipa::path(
    get,
    path = "/payments/list",
    params(
        ("customer_id" = String, Query, description = "The identifier for the customer"),
        ("starting_after" = String, Query, description = "A cursor for use in pagination, fetch the next list after some object"),
        ("ending_before" = String, Query, description = "A cursor for use in pagination, fetch the previous list before some object"),
        ("limit" = i64, Query, description = "Limit on the number of objects to return"),
        ("created" = PrimitiveDateTime, Query, description = "The time at which payment is created"),
        ("created_lt" = PrimitiveDateTime, Query, description = "Time less than the payment created time"),
        ("created_gt" = PrimitiveDateTime, Query, description = "Time greater than the payment created time"),
        ("created_lte" = PrimitiveDateTime, Query, description = "Time less than or equals to the payment created time"),
        ("created_gte" = PrimitiveDateTime, Query, description = "Time greater than or equals to the payment created time")
    ),
    responses(
        (status = 200, description = "Received payment list"),
        (status = 404, description = "No payments found")
    ),
    tag = "Payments",
    operation_id = "List all Payments",
    security(("api_key" = []))
)]
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
        state.get_ref(),
        &req,
        payload,
        |state, merchant_account, req| {
            payments::list_payments(&*state.store, merchant_account, req)
        },
        &auth::ApiKeyAuth,
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
