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
pub async fn receive_incoming_webhook<W: types::OutgoingWebhookType>(
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
        |state, auth, body| {
            webhooks::webhooks_core::<W>(
                state,
                &req,
                auth.merchant_account,
                auth.key_store,
                &connector_name,
                body,
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    )
    .await
}
