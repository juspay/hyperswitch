//! The notify routes, exercised through actix as a caller would reach them.
//!
//! These go through the real route tree rather than calling handlers directly, because most of what
//! this ticket decided lives *between* the handler and the caller: the guard, the path extractor,
//! the body extractor's rejection shape, and which outcomes are a `200` versus an error.
//!
//! Both destinations are `log` destinations, so nothing here reaches a network.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::{collections::HashMap, sync::Arc};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use external_services::email::no_email::NoEmailClient;
use observability::{
    auth::X_INTERNAL_API_KEY,
    domain::notifier::{
        chat::{ChatNotifier, LogChatNotifier},
        email::{EmailNotifier, EmailServiceNotifier},
        Registry,
    },
    routes::Alerts,
    state::AppState,
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";
const CHAT: &str = "sr_alerts";
const EMAIL: &str = "oncall";

async fn state() -> AppState {
    state_with_max(25 * 1024 * 1024).await
}

async fn state_with_max(max_upload_bytes: usize) -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
        "chat": { "max_upload_bytes": max_upload_bytes }
    }))
    .expect("the test configuration should deserialize");

    let chat: Arc<dyn ChatNotifier> = Arc::new(LogChatNotifier::new(CHAT.to_owned()));
    // The real notifier over `NoEmailClient`, which accepts and logs. Exercises the composition
    // path rather than a stand-in, and needs no credentials.
    let email: Arc<dyn EmailNotifier> = Arc::new(EmailServiceNotifier::new(
        EMAIL.to_owned(),
        Arc::new(Box::new(NoEmailClient::create().await)),
        serde_json::from_value(json!("oncall@example.com")).expect("a valid recipient"),
        None,
    ));

    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::new(HashMap::from([(CHAT.to_owned(), chat)]))),
        email: Arc::new(Registry::new(HashMap::from([(EMAIL.to_owned(), email)]))),
    }
}

async fn call(request: TestRequest) -> (StatusCode, Value) {
    call_with_state(request, state().await).await
}

async fn call_with_state(request: TestRequest, state: AppState) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state))).await;
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = test::read_body(response).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

fn post(uri: &str, body: Value) -> TestRequest {
    TestRequest::post()
        .uri(uri)
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .set_json(body)
}

fn upload(body: impl Into<Vec<u8>>) -> TestRequest {
    TestRequest::post()
        .uri(&format!("/alerts/chat/upload/{CHAT}"))
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .insert_header(("content-type", "multipart/form-data; boundary=BOUNDARY"))
        .set_payload(actix_web::web::Bytes::from(body.into()))
}

fn upload_body(file: &str) -> String {
    format!(
        "--BOUNDARY\r\nContent-Disposition: form-data; name=\"file\"; filename=\"report.pdf\"\r\nContent-Type: application/pdf\r\n\r\n{file}\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nDaily report\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=\"reply_to\"\r\n\r\n1.2\r\n--BOUNDARY--\r\n"
    )
}

#[actix_web::test]
async fn a_chat_notification_reports_delivery_and_a_message_id() {
    let (status, body) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": "*3 merchants not converting*" }),
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "delivered");
    assert!(body["message_id"].as_str().is_some_and(|id| !id.is_empty()));
    assert!(body.get("error_code").is_none());
}

/// The threading round trip a recovery notice needs: post, keep the id, reply under it.
#[actix_web::test]
async fn a_chat_notification_can_reply_under_an_earlier_message() {
    let (_, first) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": "alerts" }),
    ))
    .await;

    let (status, second) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": "recovered", "reply_to": first["message_id"] }),
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["status"], "delivered");
}

#[actix_web::test]
async fn a_chat_file_can_be_uploaded_into_a_thread() {
    let (status, body) = call(upload(upload_body("%PDF-test"))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "delivered");
    assert!(body["file_id"].as_str().is_some_and(|id| !id.is_empty()));
    assert!(body.get("message_id").is_none());
}

#[actix_web::test]
async fn an_upload_is_refused_while_streaming_past_the_configured_cap() {
    let (status, body) =
        call_with_state(upload(upload_body("four")), state_with_max(3).await).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_unparseable_upload_uses_the_standard_bad_request_shape() {
    let request = TestRequest::post()
        .uri(&format!("/alerts/chat/upload/{CHAT}"))
        .insert_header(("content-type", "multipart/form-data; boundary=BOUNDARY"))
        .set_payload("not multipart");
    let (status, body) = call(request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_upload_rejects_unknown_fields() {
    let body = upload_body("%PDF").replace("name=\"title\"", "name=\"titel\"");
    let (status, body) = call(upload(body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_email_notification_reports_delivery() {
    let (status, body) = call(post(
        &format!("/alerts/email/notify/{EMAIL}"),
        json!({ "subject": "3 merchants not converting", "body": "<pre>...</pre>" }),
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "delivered");
}

/// The load-bearing property of the response shape: a caller cannot read a `200` and assume the
/// message arrived, because `status` is always there to be checked.
#[actix_web::test]
async fn every_success_carries_a_status() {
    for (uri, body) in [
        (
            format!("/alerts/chat/notify/{CHAT}"),
            json!({ "text": "x" }),
        ),
        (
            format!("/alerts/email/notify/{EMAIL}"),
            json!({ "subject": "s", "body": "b" }),
        ),
    ] {
        let (status, body) = call(post(&uri, body)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.get("status").is_some(), "{uri} returned no status");
    }
}

/// Threading against a mailing list is a caller bug, and `deny_unknown_fields` makes it loud rather
/// than a field that quietly goes nowhere.
#[actix_web::test]
async fn threading_against_an_email_destination_is_rejected() {
    let (status, body) = call(post(
        &format!("/alerts/email/notify/{EMAIL}"),
        json!({ "subject": "s", "body": "b", "reply_to": "cmtk931s1" }),
    ))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_unknown_destination_is_a_404() {
    let (status, body) = call(post("/alerts/chat/notify/typo", json!({ "text": "x" }))).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "IR_02");
}

/// The id a caller guessed must not come back, and nor must the ones that exist.
#[actix_web::test]
async fn an_unknown_destination_does_not_echo_or_enumerate() {
    let (_, body) = call(post("/alerts/chat/notify/typo", json!({ "text": "x" }))).await;

    let rendered = body.to_string();
    assert!(!rendered.contains("typo"));
    assert!(!rendered.contains(CHAT));
}

/// A malformed body must render like every other error, not as actix's own plain-text 400.
#[actix_web::test]
async fn a_missing_field_renders_in_our_error_shape() {
    let (status, body) = call(post(&format!("/alerts/chat/notify/{CHAT}"), json!({}))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}

/// Serde's message quotes the body, and a body carries merchant ids and payment volumes. It goes to
/// the log; the caller gets the code.
#[actix_web::test]
async fn a_parse_failure_does_not_echo_the_body_back() {
    let (_, body) = call(post(
        &format!("/alerts/chat/notify/{CHAT}"),
        json!({ "text": 12345 }),
    ))
    .await;

    assert!(!body.to_string().contains("12345"));
}

#[actix_web::test]
async fn both_routes_are_behind_the_guard() {
    for (uri, body) in [
        (
            format!("/alerts/chat/notify/{CHAT}"),
            json!({ "text": "x" }),
        ),
        (
            format!("/alerts/email/notify/{EMAIL}"),
            json!({ "subject": "s", "body": "b" }),
        ),
    ] {
        let (status, body) = call(TestRequest::post().uri(&uri).set_json(body)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} must be guarded");
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

/// The guard runs before the destination is resolved, so an unauthenticated caller cannot probe
/// which ids exist by watching the status change.
#[actix_web::test]
async fn a_bad_key_is_rejected_before_the_destination_is_resolved() {
    let (known, _) = call(
        TestRequest::post()
            .uri(&format!("/alerts/chat/notify/{CHAT}"))
            .insert_header((X_INTERNAL_API_KEY, "wrong"))
            .set_json(json!({ "text": "x" })),
    )
    .await;

    let (unknown, _) = call(
        TestRequest::post()
            .uri("/alerts/chat/notify/typo")
            .insert_header((X_INTERNAL_API_KEY, "wrong"))
            .set_json(json!({ "text": "x" })),
    )
    .await;

    assert_eq!(known, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown, StatusCode::UNAUTHORIZED);
}

/// **Known and accepted:** actix runs the body extractor before the handler, and the guard runs
/// inside the handler via `server_wrap`, so an unparseable body is answered 400 without its key
/// being checked. Restoring "guard strictly first" means extracting raw bytes and deserializing by
/// hand, trading typed extraction for the concealment of a documented schema. Kept as a test so the
/// ordering is a recorded property rather than something a reviewer rediscovers.
#[actix_web::test]
async fn an_unparseable_body_is_rejected_before_the_key_is_checked() {
    let (status, body) = call(
        TestRequest::post()
            .uri(&format!("/alerts/chat/notify/{CHAT}"))
            .set_json(json!({})),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}
