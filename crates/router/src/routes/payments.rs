use crate::core::api_locking::{self, GetLockingInput};
pub mod helpers;

use actix_web::{web, Responder};
use api_models::payments::HeaderPayload;
use error_stack::report;
use router_env::{instrument, tracing, types, Flow};

use crate::{
    self as app,
    core::{
        errors::http_not_implemented,
        payment_methods::{Oss, PaymentMethodRetrieve},
        payments::{self, PaymentRedirectFlow},
    },
    // openapi::examples::{
    //     PAYMENTS_CREATE, PAYMENTS_CREATE_MINIMUM_FIELDS, PAYMENTS_CREATE_WITH_ADDRESS,
    //     PAYMENTS_CREATE_WITH_CUSTOMER_DATA, PAYMENTS_CREATE_WITH_FORCED_3DS,
    //     PAYMENTS_CREATE_WITH_MANUAL_CAPTURE, PAYMENTS_CREATE_WITH_NOON_ORDER_CATETORY,
    //     PAYMENTS_CREATE_WITH_ORDER_DETAILS,
    // },
    routes::lock_utils,
    services::{api, authentication as auth},
    types::{
        api::{self as api_types, enums as api_enums, payments as payment_types},
        domain,
        transformers::ForeignTryFrom,
    },
};

/// Payments - Create
///
/// To process a payment you will have to create a payment, attach a payment method and confirm. Depending on the user journey you wish to achieve, you may opt to all the steps in a single request or in a sequence of API request using following APIs: (i) Payments - Update, (ii) Payments - Confirm, and (iii) Payments - Capture
#[utoipa::path(
    post,
    path = "/payments",
    request_body(
        content = PaymentsCreateRequest,
        // examples(
        //     (
        //         "Create a payment with minimul fields" = (
        //             value = json!(PAYMENTS_CREATE_MINIMUM_FIELDS)
        //         )
        //     ),
        //     (
        //         "Create a manual capture payment" = (
        //             value = json!(PAYMENTS_CREATE_WITH_MANUAL_CAPTURE)
        //         )
        //     ),
        //     (
        //         "Create a payment with address" = (
        //             value = json!(PAYMENTS_CREATE_WITH_ADDRESS)
        //         )
        //     ),
        //     (
        //         "Create a payment with customer details" = (
        //             value = json!(PAYMENTS_CREATE_WITH_CUSTOMER_DATA)
        //         )
        //     ),
        //     (
        //         "Create a 3DS payment" = (
        //             value = json!(PAYMENTS_CREATE_WITH_FORCED_3DS)
        //         )
        //     ),
        //     (
        //         "Create a payment" = (
        //             value = json!(PAYMENTS_CREATE)
        //         )
        //     ),
        //     (
        //         "Create a payment with order details" = (
        //             value = json!(PAYMENTS_CREATE_WITH_ORDER_DETAILS)
        //         )
        //     ),
        //     (
        //         "Create a payment with order category for noon" = (
        //             value = json!(PAYMENTS_CREATE_WITH_NOON_ORDER_CATETORY)
        //         )
        //     ),
        // )
    ),
    responses(
        (status = 200, description = "Payment created", body = PaymentsResponse),
        (status = 400, description = "Missing Mandatory fields")
    ),
    tag = "Payments",
    operation_id = "Create a Payment",
    security(("api_key" = [])),
)]
#[instrument(skip_all, fields(flow = ?Flow::PaymentsCreate))]
pub async fn payments_create(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentsRequest>,
) -> impl Responder {
    let flow = Flow::PaymentsCreate;
    let payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    let locking_action = payload.get_locking_input(flow.clone());

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            authorize_verify_select::<_, Oss>(
                payments::PaymentCreate,
                state,
                auth.merchant_account,
                auth.key_store,
                payment_types::HeaderPayload::default(),
                req,
                api::AuthFlow::Merchant,
            )
        },
        &auth::ApiKeyAuth,
        locking_action,
    )
    .await
}
// /// Payments - Redirect
// ///
// /// For a payment which involves the redirection flow. This redirects the user to the authentication page
// #[utoipa::path(
//     get,
//     path = "/payments/redirect/{payment_id}/{merchant_id}/{attempt_id}",
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
#[instrument(skip(state, req), fields(flow = ?Flow::PaymentsStart))]
pub async fn payments_start(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let flow = Flow::PaymentsStart;
    let (payment_id, merchant_id, attempt_id) = path.into_inner();
    let payload = payment_types::PaymentsStartRequest {
        payment_id: payment_id.clone(),
        merchant_id: merchant_id.clone(),
        attempt_id: attempt_id.clone(),
    };

    let locking_action = payload.get_locking_input(flow.clone());

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            payments::payments_core::<
                api_types::Authorize,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                Oss,
            >(
                state,
                auth.merchant_account,
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
#[instrument(skip(state, req), fields(flow = ?Flow::PaymentsRetrieve))]
// #[get("/{payment_id}")]
pub async fn payments_retrieve(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    json_payload: web::Query<payment_types::PaymentRetrieveBody>,
) -> impl Responder {
    let flow = Flow::PaymentsRetrieve;
    let payload = payment_types::PaymentsRetrieveRequest {
        resource_id: payment_types::PaymentIdType::PaymentIntentId(path.to_string()),
        merchant_id: json_payload.merchant_id.clone(),
        force_sync: json_payload.force_sync.unwrap_or(false),
        client_secret: json_payload.client_secret.clone(),
        expand_attempts: json_payload.expand_attempts,
        expand_captures: json_payload.expand_captures,
        ..Default::default()
    };
    let (auth_type, auth_flow) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(report!(err)),
        };

    let locking_action = payload.get_locking_input(flow.clone());

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            payments::payments_core::<api_types::PSync, payment_types::PaymentsResponse, _, _, _,Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentStatus,
                req,
                auth_flow,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
    )
    .await
}
/// Payments - Retrieve with gateway credentials
///
/// To retrieve the properties of a Payment. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[utoipa::path(
    post,
    path = "/sync",
    request_body=PaymentRetrieveBodyWithCredentials,
    responses(
        (status = 200, description = "Gets the payment with final status", body = PaymentsResponse),
        (status = 404, description = "No payment found")
    ),
    tag = "Payments",
    operation_id = "Retrieve a Payment",
    security(("api_key" = []))
)]
#[instrument(skip(state, req), fields(flow = ?Flow::PaymentsRetrieve))]
// #[post("/sync")]
pub async fn payments_retrieve_with_gateway_creds(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: web::Json<payment_types::PaymentRetrieveBodyWithCredentials>,
) -> impl Responder {
    let (auth_type, _auth_flow) = match auth::get_auth_type_and_flow(req.headers()) {
        Ok(auth) => auth,
        Err(err) => return api::log_and_return_error_response(report!(err)),
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
    let flow = Flow::PaymentsRetrieve;

    let locking_action = payload.get_locking_input(flow.clone());

    api::server_wrap(
        flow,
        state,
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
                HeaderPayload::default(),
            )
        },
        &*auth_type,
        locking_action,
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
    let flow = Flow::PaymentsUpdate;
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

    let locking_action = payload.get_locking_input(flow.clone());

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            authorize_verify_select::<_, Oss>(
                payments::PaymentUpdate,
                state,
                auth.merchant_account,
                auth.key_store,
                payment_types::HeaderPayload::default(),
                req,
                auth_flow,
            )
        },
        &*auth_type,
        locking_action,
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
    let flow = Flow::PaymentsConfirm;
    let mut payload = json_payload.into_inner();

    if let Some(api_enums::CaptureMethod::Scheduled) = payload.capture_method {
        return http_not_implemented();
    };

    if let Err(err) = helpers::populate_ip_into_browser_info(&req, &mut payload) {
        return api::log_and_return_error_response(err);
    }

    let payment_id = path.into_inner();
    payload.payment_id = Some(payment_types::PaymentIdType::PaymentIntentId(payment_id));
    payload.confirm = Some(true);
    let header_payload = match payment_types::HeaderPayload::foreign_try_from(req.headers()) {
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

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            authorize_verify_select::<_, Oss>(
                payments::PaymentConfirm,
                state,
                auth.merchant_account,
                auth.key_store,
                header_payload,
                req,
                auth_flow,
            )
        },
        &*auth_type,
        locking_action,
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
    let flow = Flow::PaymentsCapture;
    let payload = payment_types::PaymentsCaptureRequest {
        payment_id: path.into_inner(),
        ..json_payload.into_inner()
    };

    let locking_action = payload.get_locking_input(flow.clone());

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, payload| {
            payments::payments_core::<
                api_types::Capture,
                payment_types::PaymentsResponse,
                _,
                _,
                _,
                Oss,
            >(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentCapture,
                payload,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &auth::ApiKeyAuth,
        locking_action,
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
    let flow = Flow::PaymentsSessionToken;
    let payload = json_payload.into_inner();

    let locking_action = payload.get_locking_input(flow.clone());

    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, payload| {
            payments::payments_core::<
                api_types::Session,
                payment_types::PaymentsSessionResponse,
                _,
                _,
                _,
                Oss,
            >(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentSession,
                payload,
                api::AuthFlow::Client,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &auth::PublishableKeyAuth,
        locking_action,
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
    json_payload: Option<web::Form<serde_json::Value>>,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let flow = Flow::PaymentsRedirect;
    let (payment_id, merchant_id, connector) = path.into_inner();
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
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            <payments::PaymentRedirectSync as PaymentRedirectFlow<Oss>>::handle_payments_redirect_response(
                &payments::PaymentRedirectSync {},
                state,
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

// /// Payments - Redirect response with creds_identifier
// ///
// /// To get the payment response for redirect flows
// #[utoipa::path(
//     post,
//     path = "/payments/{payment_id}/{merchant_id}/response/{connector}/{cred_identifier}",
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
pub async fn payments_redirect_response_with_creds_identifier(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(String, String, String, String)>,
) -> impl Responder {
    let (payment_id, merchant_id, connector, creds_identifier) = path.into_inner();
    let param_string = req.query_string();

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
        |state, auth, req| {
           <payments::PaymentRedirectSync as PaymentRedirectFlow<Oss>>::handle_payments_redirect_response(
                &payments::PaymentRedirectSync {},
                state,
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
#[instrument(skip_all)]
pub async fn payments_complete_authorize(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    json_payload: Option<web::Form<serde_json::Value>>,
    path: web::Path<(String, String, String)>,
) -> impl Responder {
    let flow = Flow::PaymentsRedirect;
    let (payment_id, merchant_id, connector) = path.into_inner();
    let param_string = req.query_string();

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
        |state, auth, req| {

            <payments::PaymentRedirectCompleteAuthorize as PaymentRedirectFlow<Oss>>::handle_payments_redirect_response(
                &payments::PaymentRedirectCompleteAuthorize {},
                state,
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
    let flow = Flow::PaymentsCancel;
    let mut payload = json_payload.into_inner();
    let payment_id = path.into_inner();
    payload.payment_id = payment_id;
    let locking_action = payload.get_locking_input(flow.clone());
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| {
            payments::payments_core::<api_types::Void, payment_types::PaymentsResponse, _, _, _,Oss>(
                state,
                auth.merchant_account,
                auth.key_store,
                payments::PaymentCancel,
                req,
                api::AuthFlow::Merchant,
                payments::CallConnectorAction::Trigger,
                None,
                HeaderPayload::default(),
            )
        },
        &auth::ApiKeyAuth,
        locking_action,
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
pub async fn payments_list(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<payment_types::PaymentListConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| payments::list_payments(state, auth.merchant_account, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(feature = "olap")]
pub async fn payments_list_by_filter(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<payment_types::PaymentListFilterConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| payments::apply_filters_on_payments(state, auth.merchant_account, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
#[instrument(skip_all, fields(flow = ?Flow::PaymentsList))]
#[cfg(feature = "olap")]
pub async fn get_filters_for_payments(
    state: web::Data<app::AppState>,
    req: actix_web::HttpRequest,
    payload: web::Json<payment_types::TimeRange>,
) -> impl Responder {
    let flow = Flow::PaymentsList;
    let payload = payload.into_inner();
    api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth, req| payments::get_filters_for_payments(state, auth.merchant_account, req),
        &auth::ApiKeyAuth,
        api_locking::LockAction::NotApplicable,
    )
    .await
}
async fn authorize_verify_select<Op, Ctx>(
    operation: Op,
    state: app::AppState,
    merchant_account: domain::MerchantAccount,
    key_store: domain::MerchantKeyStore,
    header_payload: HeaderPayload,
    req: api_models::payments::PaymentsRequest,
    auth_flow: api::AuthFlow,
) -> app::core::errors::RouterResponse<api_models::payments::PaymentsResponse>
where
    Ctx: PaymentMethodRetrieve,
    Op: Sync
        + Clone
        + std::fmt::Debug
        + payments::operations::Operation<
            api_types::Authorize,
            api_models::payments::PaymentsRequest,
            Ctx,
        > + payments::operations::Operation<
            api_types::SetupMandate,
            api_models::payments::PaymentsRequest,
            Ctx,
        >,
{
    // TODO: Change for making it possible for the flow to be inferred internally or through validation layer
    // This is a temporary fix.
    // After analyzing the code structure,
    // the operation are flow agnostic, and the flow is only required in the post_update_tracker
    // Thus the flow can be generated just before calling the connector instead of explicitly passing it here.

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
                Ctx,
            >(
                state,
                merchant_account,
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
                Ctx,
            >(
                state,
                merchant_account,
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
                        unique_locking_key: id.to_owned(),
                        api_identifier: lock_utils::ApiIdentifier::from(flow),
                        override_lock_retries: None,
                    },
                }
            }
            _ => api_locking::LockAction::NotApplicable,
        }
    }
}

impl GetLockingInput for payment_types::PaymentsStartRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

impl GetLockingInput for payment_types::PaymentsRetrieveRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        match self.resource_id {
            payment_types::PaymentIdType::PaymentIntentId(ref id) => {
                api_locking::LockAction::Hold {
                    input: api_locking::LockingInput {
                        unique_locking_key: id.to_owned(),
                        api_identifier: lock_utils::ApiIdentifier::from(flow),
                        override_lock_retries: None,
                    },
                }
            }
            _ => api_locking::LockAction::NotApplicable,
        }
    }
}

impl GetLockingInput for payment_types::PaymentsSessionRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

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
                        unique_locking_key: id.to_owned(),
                        api_identifier: lock_utils::ApiIdentifier::from(flow),
                        override_lock_retries: None,
                    },
                }
            }
            _ => api_locking::LockAction::NotApplicable,
        }
    }
}

impl GetLockingInput for payment_types::PaymentsCancelRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}

impl GetLockingInput for payment_types::PaymentsCaptureRequest {
    fn get_locking_input<F>(&self, flow: F) -> api_locking::LockAction
    where
        F: types::FlowMetric,
        lock_utils::ApiIdentifier: From<F>,
    {
        api_locking::LockAction::Hold {
            input: api_locking::LockingInput {
                unique_locking_key: self.payment_id.to_owned(),
                api_identifier: lock_utils::ApiIdentifier::from(flow),
                override_lock_retries: None,
            },
        }
    }
}
