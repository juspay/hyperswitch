use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        payment_methods::Oss,
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
    let (merchant_id, connector_id_or_name) = path.into_inner();

    let body_vec: Vec<u8> = body.to_vec();
    let payload: serde_json::Value = serde_json::from_slice(&body_vec)
        .into_report()
        .change_context(errors::ApiErrorResponse::WebhookProcessingFailure)?;

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        payload,
        |state, auth, _| {
            webhooks::webhooks_wrapper::<W, Oss>(
                &flow,
                state.to_owned(),
                &req,
                auth.merchant_account,
                auth.key_store,
                &connector_id_or_name,
                body.clone(),
            )
        },
        &auth::MerchantIdAuth(merchant_id),
        api_locking::LockAction::NotApplicable,
    ))
    .await
}
