use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        webhooks::{self, types},
    },
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::IncomingWebhookReceive))]
#[cfg(feature = "v1")]
pub async fn receive_incoming_webhook<W: types::OutgoingWebhookType>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(common_utils::id_type::MerchantId, String)>,
) -> impl Responder {
    let flow = Flow::IncomingWebhookReceive;
    let (merchant_id, connector_id_or_name) = path.into_inner();

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        (),
        |state, auth, _, req_state| {
            webhooks::incoming_webhooks_wrapper::<W>(
                &flow,
                state.to_owned(),
                req_state,
                &req,
                auth.merchant_account,
                auth.key_store,
                &connector_id_or_name,
                body.clone(),
                false,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v1")]
#[instrument(skip_all, fields(flow = ?Flow::IncomingRelayWebhookReceive))]
pub async fn receive_incoming_relay_webhook<W: types::OutgoingWebhookType>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::MerchantConnectorAccountId,
    )>,
) -> impl Responder {
    let flow = Flow::IncomingWebhookReceive;
    let (merchant_id, connector_id) = path.into_inner();
    let is_relay_webhook = true;

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        (),
        |state, auth, _, req_state| {
            webhooks::incoming_webhooks_wrapper::<W>(
                &flow,
                state.to_owned(),
                req_state,
                &req,
                auth.merchant_account,
                auth.key_store,
                connector_id.get_string_repr(),
                body.clone(),
                is_relay_webhook,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[cfg(feature = "v2")]
#[instrument(skip_all, fields(flow = ?Flow::IncomingRelayWebhookReceive))]
pub async fn receive_incoming_relay_webhook<W: types::OutgoingWebhookType>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
        common_utils::id_type::MerchantConnectorAccountId,
    )>,
) -> impl Responder {
    let flow = Flow::IncomingWebhookReceive;
    let (merchant_id, profile_id, connector_id) = path.into_inner();
    let is_relay_webhook = true;

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        (),
        |state, auth, _, req_state| {
            webhooks::incoming_webhooks_wrapper::<W>(
                &flow,
                state.to_owned(),
                req_state,
                &req,
                auth.merchant_account,
                auth.profile,
                auth.key_store,
                &connector_id,
                body.clone(),
                is_relay_webhook,
            )
        },
        &auth::MerchantIdAndProfileIdAuth {
            merchant_id,
            profile_id,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}

#[instrument(skip_all, fields(flow = ?Flow::IncomingWebhookReceive))]
#[cfg(feature = "v2")]
pub async fn receive_incoming_webhook<W: types::OutgoingWebhookType>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(
        common_utils::id_type::MerchantId,
        common_utils::id_type::ProfileId,
        common_utils::id_type::MerchantConnectorAccountId,
    )>,
) -> impl Responder {
    let flow = Flow::IncomingWebhookReceive;
    let (merchant_id, profile_id, connector_id) = path.into_inner();

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        (),
        |state, auth, _, req_state| {
            webhooks::incoming_webhooks_wrapper::<W>(
                &flow,
                state.to_owned(),
                req_state,
                &req,
                auth.merchant_account,
                auth.profile,
                auth.key_store,
                &connector_id,
                body.clone(),
                false,
            )
        },
        &auth::MerchantIdAndProfileIdAuth {
            merchant_id,
            profile_id,
        },
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
