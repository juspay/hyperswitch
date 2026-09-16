//! The merchant alert delivery switch routes, exercised through actix as a caller would reach
//! them.
//!
//! The store is built with no idle connections, so only what fails *before* the database is
//! reached can be tested here: the guard, and every `400` a bad request earns before a query is
//! ever issued.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::Arc;

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use observability::{
    auth::X_INTERNAL_API_KEY, db::Store, domain::notifier::Registry, routes::Alerts,
    settings::Database, state::AppState,
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";
const BASE: &str = "/alerts/alerts_manager/external_config";

async fn state() -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY }
    }))
    .expect("the test configuration should deserialize");

    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::default()),
        email: Arc::new(Registry::default()),
        metrics: None,
        store: Arc::new(lazy_store().await),
    }
}

/// A store that never connects. No idle connections are opened at build, and none of the failures
/// tested here reach the database, so these tests need no database.
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

fn authed(request: TestRequest) -> TestRequest {
    request.insert_header((X_INTERNAL_API_KEY, API_KEY))
}

fn post(uri: &str, body: Value) -> TestRequest {
    authed(TestRequest::post().uri(uri)).set_json(body)
}

fn get(uri: &str) -> TestRequest {
    authed(TestRequest::get().uri(uri))
}

#[actix_web::test]
async fn every_route_is_behind_the_guard() {
    for request in [
        TestRequest::post()
            .uri(BASE)
            .set_json(json!({ "name": "Zero Volume", "product": "payments" })),
        TestRequest::post()
            .uri(&format!("{BASE}/list"))
            .set_json(json!({})),
        TestRequest::get().uri(&format!("{BASE}/Zero%20Volume/payments")),
        TestRequest::post()
            .uri(&format!("{BASE}/Zero%20Volume/payments"))
            .set_json(json!({ "is_enabled": false })),
        TestRequest::delete().uri(&format!("{BASE}/Zero%20Volume/payments")),
    ] {
        let (status, body) = call(request).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

#[actix_web::test]
async fn a_malformed_or_unknown_field_body_renders_in_our_error_shape() {
    for request in [
        post(
            BASE,
            json!({ "name": "Zero Volume", "product": "payments", "typo": true }),
        ),
        post(&format!("{BASE}/list"), json!({ "format": "csv" })),
        post(
            &format!("{BASE}/Zero%20Volume/payments"),
            json!({ "name": "Zero Volume" }),
        ),
    ] {
        let (status, body) = call(request).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"]["code"], "IR_04");
    }
}

#[actix_web::test]
async fn an_invalid_create_is_refused_before_the_database() {
    let (status, body) = call(post(BASE, json!({ "name": "  ", "product": "payments" }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn get_on_list_is_not_allowed() {
    let (status, _) = call(get(&format!("{BASE}/list"))).await;

    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}
