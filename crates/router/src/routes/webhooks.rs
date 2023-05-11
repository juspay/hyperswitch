use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::webhooks,
    services::{api, authentication as auth},
    types::api as api_types,
};

/// Webhooks - Accept
///
/// Endpoint to accept incoming webhooks"
#[utoipa::path(
    post,
    path = "/webhooks/{merchant_id}/{connector}",
    request_body = String,
    responses(
        (status = 200, description = "Webhook Processed successfully"),
    ),
    tag = "Process Webhook",
    operation_id = "Process Webhook",
    security(("admin_api_key" = []))
)]
#[instrument(skip_all, fields(flow = ?Flow::IncomingWebhookReceive))]
pub async fn receive_incoming_webhook<W: api_types::OutgoingWebhookType>(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let flow = Flow::IncomingWebhookReceive;
    let (merchant_id, connector_name) = path.into_inner();

    api::server_wrap(
        flow,
        state.get_ref(),
        &req,
        body,
        |state, merchant_account, body| {
            webhooks::webhooks_core::<W>(state, &req, merchant_account, &connector_name, body)
        },
        &auth::MerchantIdAuth(merchant_id),
    )
    .await
}
