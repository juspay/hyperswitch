//! The notification_reads routes, exercised through actix as a caller would reach them.
//!
//! A request that passes both the guard and validation would go on to lease a database
//! connection, which this lazy store never provides, so only auth and 400s are covered here. The
//! 200/404 behaviour is checked manually against a real database (see the plan).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::Arc;

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::Store,
    domain::notifier::{chat::ChatNotifier, email::EmailNotifier, Registry},
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";
const BASE: &str = "/alerts/alerts_manager/notification_reads";

async fn state() -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY }
    }))
    .expect("the test configuration should deserialize");

    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::<dyn ChatNotifier>::default()),
        email: Arc::new(Registry::<dyn EmailNotifier>::default()),
        metrics: None,
        store: Arc::new(lazy_store().await),
    }
}

/// A store that never connects. No idle connections are opened at build, and a request that never
/// passes both the guard and validation never asks for one, so these tests need no database.
async fn lazy_store() -> Store {
    Store::new(&Database {
        username: "unused".to_owned(),
        host: "localhost".to_owned(),
        dbname: "unused".to_owned(),
        min_idle_pool_size: 0,
        ..Default::default()
    })
    .await
    .expect("a pool with no idle connections builds without a database")
}

async fn call(request: TestRequest) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state().await))).await;
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = test::read_body(response).await;

    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

fn get(uri: &str) -> TestRequest {
    TestRequest::get()
        .uri(uri)
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
}

fn post(uri: &str, body: Value) -> TestRequest {
    TestRequest::post()
        .uri(uri)
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
        .set_json(body)
}

#[actix_web::test]
async fn both_routes_are_behind_the_guard() {
    let (get_status, get_body) =
        call(TestRequest::get().uri(&format!("{BASE}?user_name=ops.engineer%40example.com"))).await;
    let (post_status, post_body) = call(
        TestRequest::post()
            .uri(BASE)
            .set_json(json!({ "user_name": "ops.engineer@example.com" })),
    )
    .await;

    assert_eq!(get_status, StatusCode::UNAUTHORIZED);
    assert_eq!(get_body["error"]["code"], "IR_01");
    assert_eq!(post_status, StatusCode::UNAUTHORIZED);
    assert_eq!(post_body["error"]["code"], "IR_01");
}

#[actix_web::test]
async fn a_retrieve_without_user_name_renders_in_our_error_shape() {
    let (status, body) = call(get(BASE)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn a_retrieve_with_an_unknown_query_field_is_refused() {
    let (status, body) = call(get(&format!(
        "{BASE}?user_name=ops.engineer%40example.com&extra=1"
    )))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_upsert_without_user_name_renders_in_our_error_shape() {
    let (status, body) = call(post(BASE, json!({}))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_upsert_cannot_choose_the_time() {
    let (status, body) = call(post(
        BASE,
        json!({ "user_name": "ops.engineer@example.com", "last_read_at": "2026-09-15T09:42:10.512Z" }),
    ))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_over_long_user_name_is_refused_before_the_database() {
    let user_name = "a".repeat(256);
    let (status, body) = call(post(BASE, json!({ "user_name": user_name }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}
