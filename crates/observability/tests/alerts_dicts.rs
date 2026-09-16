//! The mappers dictionary routes, exercised through actix as a caller would reach them.
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
use api_models::observability::alert_manager::alert_dicts::AlertsDictsCreateRequest;
use observability::{
    auth::X_INTERNAL_API_KEY, db::Store, domain::notifier::Registry, routes::Alerts,
    settings::Database, state::AppState,
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";
const BASE: &str = "/alerts/alerts_manager/dicts";

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

fn post(body: Value) -> TestRequest {
    authed(TestRequest::post().uri(BASE)).set_json(body)
}

fn get(uri: &str) -> TestRequest {
    authed(TestRequest::get().uri(uri))
}

fn delete(uri: &str) -> TestRequest {
    authed(TestRequest::delete().uri(uri))
}

#[actix_web::test]
async fn all_routes_are_behind_the_guard() {
    for request in [
        TestRequest::post()
            .uri(BASE)
            .set_json(json!({ "name": "t", "key": "k", "values_": ["v"] })),
        TestRequest::get().uri(BASE),
        TestRequest::get().uri(&format!("{BASE}/abc")),
        TestRequest::delete().uri(&format!("{BASE}/abc")),
    ] {
        let (status, body) = call(request).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

#[actix_web::test]
async fn a_save_without_values_is_refused() {
    let (status, body) = call(post(json!({ "name": "t", "key": "k" }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[test]
fn a_save_ignores_ts_created() {
    let request: AlertsDictsCreateRequest = serde_json::from_value(json!({
        "name": "t",
        "key": "k",
        "values_": ["v"],
        "ts_created": "2026-09-15T06:00:00Z"
    }))
    .expect("unknown fields are ignored by the request model");

    assert_eq!(request.name, "t");
    assert_eq!(request.key, "k");
}

#[actix_web::test]
async fn a_blank_name_is_refused_before_the_database() {
    let (status, body) = call(post(json!({ "name": " ", "key": "k", "values_": ["v"] }))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_empty_id_is_refused() {
    let (status, body) = call(get(&format!("{BASE}/%20"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");

    let (status, body) = call(delete(&format!("{BASE}/%20"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn a_bad_is_enabled_renders_in_our_error_shape() {
    let (status, body) = call(get(&format!("{BASE}?is_enabled=maybe"))).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}
