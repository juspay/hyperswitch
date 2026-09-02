//! The notify routes, exercised through actix as a caller would reach them.
//!
//! These go through the real route tree rather than calling handlers directly, because most of
//! what this ticket decided lives *between* the handler and the caller: the guard, the body
//! extractor's rejection shape, the status code a failure maps onto. A unit test on a handler
//! would step over all three.
//!
//! Both destinations are `log` destinations, so nothing here reaches a network.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::{collections::HashMap, sync::Arc};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use alerts::{
    auth::X_INTERNAL_API_KEY,
    core::notifier::{
        chat::{ChatNotifier, LogChatNotifier},
        email::{EmailNotifier, LogEmailNotifier},
        Registry,
    },
    routes::Alerts,
    state::AppState,
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";
const CHAT_DESTINATION: &str = "sr_alerts";
const EMAIL_DESTINATION: &str = "oncall";

fn state() -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
    }))
    .expect("the test configuration should deserialize");

    let chat: Arc<dyn ChatNotifier> = Arc::new(LogChatNotifier::new(CHAT_DESTINATION.to_owned()));
    let email: Arc<dyn EmailNotifier> =
        Arc::new(LogEmailNotifier::new(EMAIL_DESTINATION.to_owned()));

    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::new(HashMap::from([(
            CHAT_DESTINATION.to_owned(),
            chat,
        )]))),
        email: Arc::new(Registry::new(HashMap::from([(
            EMAIL_DESTINATION.to_owned(),
            email,
        )]))),
    }
}

/// Send a request through the whole guarded scope and return the status and parsed body.
async fn call(request: TestRequest) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state()))).await;
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = test::read_body(response).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

fn chat_request(body: Value) -> TestRequest {
    TestRequest::post()
        .uri("/alerts/chat/notify")
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .set_json(body)
}

fn email_request(body: Value) -> TestRequest {
    TestRequest::post()
        .uri("/alerts/email/notify")
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .set_json(body)
}

#[actix_web::test]
async fn a_chat_notification_returns_a_message_id() {
    let (status, body) = call(chat_request(json!({
        "destination": CHAT_DESTINATION,
        "text": "*3 merchants not converting*",
    })))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["message_id"].as_str().is_some_and(|id| !id.is_empty()));
}

/// The threading round trip R needs for recovery notices: post, keep the id, reply under it.
#[actix_web::test]
async fn a_chat_notification_can_reply_under_an_earlier_message() {
    let (_, first) = call(chat_request(json!({
        "destination": CHAT_DESTINATION,
        "text": "alerts",
    })))
    .await;

    let (status, second) = call(chat_request(json!({
        "destination": CHAT_DESTINATION,
        "text": "recovered",
        "reply_to": first["message_id"],
    })))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(second["message_id"].as_str().is_some());
}

#[actix_web::test]
async fn an_email_notification_returns_an_empty_body() {
    let (status, body) = call(email_request(json!({
        "destination": EMAIL_DESTINATION,
        "subject": "[Hyperswitch] 3 merchants not converting",
        "body": "<pre>...</pre>",
    })))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({}));
}

/// Threading against a mailing list is a caller bug. Dropping the field silently would leave a
/// recovery notice floating with nothing linking it to what it cleared, and nobody would notice.
#[actix_web::test]
async fn threading_against_an_email_destination_is_rejected() {
    let (status, body) = call(email_request(json!({
        "destination": EMAIL_DESTINATION,
        "subject": "recovered",
        "body": "<pre>...</pre>",
        "reply_to": "1503435956.000247",
    })))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_unknown_destination_is_the_callers_problem_and_names_itself() {
    let (status, body) = call(chat_request(json!({
        "destination": "typo",
        "text": "hello",
    })))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_02");
    assert_eq!(body["error"]["destination"], "typo");
}

/// A malformed body must render like every other error, not as actix's own plain-text 400.
#[actix_web::test]
async fn a_missing_field_renders_in_our_error_shape() {
    let (status, body) = call(chat_request(json!({ "destination": CHAT_DESTINATION }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}

/// Serde's message quotes the body, and a body carries merchant ids and payment volumes. It goes
/// to the log; the caller gets the code.
#[actix_web::test]
async fn a_parse_failure_does_not_echo_the_body_back() {
    let (_, body) = call(chat_request(json!({
        "destination": CHAT_DESTINATION,
        "text": 12345,
    })))
    .await;

    assert!(!body.to_string().contains("12345"));
}

#[actix_web::test]
async fn both_routes_are_behind_the_guard() {
    let unauthenticated = [
        (
            "/alerts/chat/notify",
            json!({ "destination": CHAT_DESTINATION, "text": "hello" }),
        ),
        (
            "/alerts/email/notify",
            json!({ "destination": EMAIL_DESTINATION, "subject": "s", "body": "b" }),
        ),
    ];

    for (uri, body) in unauthenticated {
        let (status, body) = call(TestRequest::post().uri(uri).set_json(body)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} must be guarded");
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

/// **Known and accepted:** actix runs the body extractor before the handler, and the guard runs
/// inside the handler via `server_wrap`, so a request with an unparseable body is answered 400
/// without its key ever being checked.
///
/// Asserted rather than fixed. Restoring "guard strictly first" means extracting the body as raw
/// bytes and deserializing by hand after authenticating, which trades actix's typed extraction and
/// its rejection handling for a leak of nothing: the response distinguishes a malformed body from
/// a well-formed one, and the body schema is public. Kept as a test so the ordering is a recorded
/// property rather than something a reviewer rediscovers.
#[actix_web::test]
async fn an_unparseable_body_is_rejected_before_the_key_is_checked() {
    let (status, body) = call(
        TestRequest::post()
            .uri("/alerts/chat/notify")
            .set_json(json!({ "destination": CHAT_DESTINATION })),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

/// The guard runs before the body is looked at, so an unauthenticated caller cannot learn which
/// destinations exist by watching the error change.
#[actix_web::test]
async fn a_bad_key_is_rejected_before_the_destination_is_resolved() {
    let (status, _) = call(
        TestRequest::post()
            .uri("/alerts/chat/notify")
            .insert_header((X_INTERNAL_API_KEY, "wrong"))
            .set_json(json!({ "destination": CHAT_DESTINATION, "text": "hello" })),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
