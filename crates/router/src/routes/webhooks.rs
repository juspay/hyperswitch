use actix_web::{web, HttpRequest, Responder};
use router_env::{instrument, tracing, Flow};

use super::app::AppState;
use crate::{
    core::{
        api_locking,
        payment_methods::Oss,
        webhooks::{self, types, utils::fetch_merchant_id_for_unified_webhooks},
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

    // The endpoint '/webhooks/{merchant_id}/{mca_id OR connector_name}' manages incoming webhooks for merchants.
    // The endpoint '/webhooks/unified/{connector_name}' is designed for handling incoming webhooks specific to the merchant onboarding flow.
    let (merchant_id_or_unified, connector_id_or_name) = path.into_inner();
    let merchant_id = if merchant_id_or_unified == "unified" {
        let mid = fetch_merchant_id_for_unified_webhooks(
            state.to_owned(),
            req.to_owned(),
            body.to_owned(),
            &connector_id_or_name,
        )
        .await;
        match mid {
            Ok(merchant_id) => merchant_id,
            Err(_) => {
                return actix_web::HttpResponse::BadRequest()
                    .content_type(mime::APPLICATION_JSON)
                    .body(
                        r#"{
                            "error": {
                                "message": "Error serializing response from connector"
                            }
                        }"#,
                    )
            }
        }
    } else {
        merchant_id_or_unified
    };

    Box::pin(api::server_wrap(
        flow.clone(),
        state,
        &req,
        WebhookBytes(body),
        |state, auth, payload| {
            webhooks::webhooks_wrapper::<W, Oss>(
                &flow,
                state.to_owned(),
                &req,
                auth.merchant_account,
                auth.key_store,
                &connector_id_or_name,
                payload.0,
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
