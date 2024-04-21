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

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        (),
        |state, auth, _, req_state| {
            webhooks::webhooks_wrapper::<W, Oss>(
                &flow,
                state.to_owned(),
                req_state,
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

#[derive(Debug)]
struct WebhookBytes(web::Bytes);

impl serde::Serialize for WebhookBytes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let payload: serde_json::Value = serde_json::from_slice(&self.0).unwrap_or_default();
        payload.serialize(serializer)
    }
}

impl common_utils::events::ApiEventMetric for WebhookBytes {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::Miscellaneous)
    }
}
