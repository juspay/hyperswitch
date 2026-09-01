//! **THROWAWAY — DO NOT MERGE.**
//!
//! A hand-poke endpoint for `external_services::chat_service`, so a real Xyne or Slack channel can
//! be hit from `curl` before anything is wired up for real. It exists on the `chatservice` branch
//! only and is dropped before the pull request: the Notifier owns this surface
//! (juspay/hyperswitch-cloud#23128, #23129), not the router.
//!
//! It is **unauthenticated and takes an arbitrary base URL**, which makes it a server-side request
//! forgery primitive. That is tolerable for a branch that never merges and for nothing else.
//!
//! The whole destination travels in the request body, so nothing is read from settings but the
//! egress proxy, and no `AppState` field is added:
//!
//! ```sh
//! curl -X POST localhost:8080/chat_probe/xyne -H 'content-type: application/json' -d '{
//!   "app_jwt": "...", "channel": "C0123456789", "text": "*hello* from `chat_service`"
//! }'
//! ```

use actix_web::{web, HttpResponse};
use external_services::chat_service::{
    slack::{SlackClient, SlackConfig},
    xyne::{XyneClient, XyneConfig},
    ChatClient, ChatMessage, MessageId,
};
use router_env::logger;

use super::app;

/// A Xyne destination plus what to say to it. `base_url`, `timeout_seconds` and
/// `max_message_chars` fall back to [`XyneConfig`]'s own defaults when omitted.
#[derive(Debug, serde::Deserialize)]
pub struct XyneProbeRequest {
    #[serde(flatten)]
    config: XyneConfig,
    text: String,
    /// Reply into an existing thread, using the `ts` a previous probe returned.
    thread_ts: Option<String>,
}

/// A Slack destination plus what to say to it.
#[derive(Debug, serde::Deserialize)]
pub struct SlackProbeRequest {
    #[serde(flatten)]
    config: SlackConfig,
    text: String,
    thread_ts: Option<String>,
}

/// `POST /chat_probe/xyne`
pub async fn probe_xyne(
    state: web::Data<app::AppState>,
    payload: web::Json<XyneProbeRequest>,
) -> HttpResponse {
    let XyneProbeRequest {
        config,
        text,
        thread_ts,
    } = payload.into_inner();

    match XyneClient::new(config, state.conf.proxy.clone()) {
        Ok(client) => post(&client, text, thread_ts).await,
        Err(error) => rejected(&error),
    }
}

/// `POST /chat_probe/slack`
pub async fn probe_slack(
    state: web::Data<app::AppState>,
    payload: web::Json<SlackProbeRequest>,
) -> HttpResponse {
    let SlackProbeRequest {
        config,
        text,
        thread_ts,
    } = payload.into_inner();

    match SlackClient::new(config, state.conf.proxy.clone()) {
        Ok(client) => post(&client, text, thread_ts).await,
        Err(error) => rejected(&error),
    }
}

async fn post(client: &dyn ChatClient, text: String, thread_ts: Option<String>) -> HttpResponse {
    let message = match thread_ts {
        Some(thread_ts) => ChatMessage::new(text).reply_to(MessageId::ts(thread_ts)),
        None => ChatMessage::new(text),
    };

    match client.post_message(message).await {
        Ok(message_id) => HttpResponse::Ok().json(serde_json::json!({
            "ok": true,
            "ts": message_id.as_ts(),
        })),
        Err(error) => {
            logger::error!(?error, "chat probe failed");
            HttpResponse::BadGateway().json(serde_json::json!({
                "ok": false,
                // The debug rendering carries the whole `error_stack` report, attachments and all,
                // which is the entire point of poking this by hand.
                "error": format!("{error:?}"),
            }))
        }
    }
}

fn rejected<E: std::fmt::Debug>(error: &E) -> HttpResponse {
    HttpResponse::BadRequest().json(serde_json::json!({
        "ok": false,
        "error": format!("{error:?}"),
    }))
}
