use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::webhooks,
    services::{api, authentication as auth},
};

#[instrument(skip_all, fields(flow = ?Flow::IncomingWebhookReceive))]
pub async fn receive_incoming_webhook(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Bytes,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (merchant_id, connector_name) = path.into_inner();

    api::server_wrap(
        state.get_ref(),
        &req,
        body,
        |state, merchant_account, body| {
            webhooks::webhooks_core(state, &req, merchant_account, &connector_name, body)
        },
        &auth::MerchantIdAuth(merchant_id),
    )
    .await
}
