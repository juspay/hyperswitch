use actix_web::{web, Responder};
use router_env::{instrument, tracing, Flow};

use crate::{
    core::{api_locking, payment_link::*},
    services::{api, authentication as auth},
    AppState,
};

/// Payments Link - Retrieve
///
/// To retrieve the properties of a Payment Link. This may be used to get the status of a previously initiated payment or next action for an ongoing payment
#[instrument(skip(state, req), fields(flow = ?Flow::PaymentLinkRetrieve))]
pub async fn payment_link_retrieve(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    json_payload: web::Query<api_models::payments::RetrievePaymentLinkRequest>,
) -> impl Responder {
    let flow = Flow::PaymentLinkRetrieve;
    let payload = json_payload.into_inner();
    let api_auth = auth::ApiKeyAuth::default();

    let (auth_type, _) =
        match auth::check_client_secret_and_get_auth(req.headers(), &payload, api_auth) {
            Ok(auth) => auth,
            Err(err) => return api::log_and_return_error_response(error_stack::report!(err)),
        };

    api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, _auth, _, _| retrieve_payment_link(state, path.clone()),
        &*auth_type,
        api_locking::LockAction::NotApplicable,
    )
    .await
}

pub async fn initiate_payment_link(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::PaymentId,
    )>,
) -> impl Responder {
    let flow = Flow::PaymentLinkInitiate;
    let (merchant_id, payment_id) = path.into_inner();

    let payload = api_models::payments::PaymentLinkInitiateRequest {
        payment_id,
        merchant_id: merchant_id.clone(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth: auth::AuthenticationData, _, _| {
            let platform = auth.into();
            initiate_payment_link_flow(
                state,
                platform,
                payload.merchant_id.clone(),
                payload.payment_id.clone(),
            )
        },
        &crate::services::authentication::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn initiate_secure_payment_link(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::PaymentId,
    )>,
) -> impl Responder {
    let flow = Flow::PaymentSecureLinkInitiate;
    let (merchant_id, payment_id) = path.into_inner();
    let payload = api_models::payments::PaymentLinkInitiateRequest {
        payment_id,
        merchant_id: merchant_id.clone(),
    };
    let headers = req.headers();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth: auth::AuthenticationData, _, _| {
            let platform = auth.into();
            initiate_secure_payment_link_flow(
                state,
                platform,
                payload.merchant_id.clone(),
                payload.payment_id.clone(),
                headers,
            )
        },
        &crate::services::authentication::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

/// Payment Link - List
///
/// To list the payment links
#[instrument(skip_all, fields(flow = ?Flow::PaymentLinkList))]
pub async fn payments_link_list(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    payload: web::Query<api_models::payments::PaymentLinkListConstraints>,
) -> impl Responder {
    let flow = Flow::PaymentLinkList;
    let payload = payload.into_inner();
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload,
        |state, auth: auth::AuthenticationData, payload, _| {
            list_payment_link(state, auth.merchant_account, payload)
        },
        &auth::HeaderAuth(auth::ApiKeyAuth {
            is_connected_allowed: false,
            is_platform_allowed: false,
        }),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

pub async fn payment_link_status(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::PaymentId,
    )>,
) -> impl Responder {
    let flow = Flow::PaymentLinkStatus;
    let (merchant_id, payment_id) = path.into_inner();

    let payload = api_models::payments::PaymentLinkInitiateRequest {
        payment_id,
        merchant_id: merchant_id.clone(),
    };
    Box::pin(api::server_wrap(
        flow,
        state,
        &req,
        payload.clone(),
        |state, auth: auth::AuthenticationData, _, _| {
            let platform = auth.into();
            get_payment_link_status(
                state,
                platform,
                payload.merchant_id.clone(),
                payload.payment_id.clone(),
            )
        },
        &crate::services::authentication::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
