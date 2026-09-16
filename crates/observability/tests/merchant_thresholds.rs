//! The per-merchant threshold override routes, exercised through actix as a caller would reach
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
const BASE: &str = "/alerts/alerts_manager/merchant_thresholds";

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

fn delete(uri: &str) -> TestRequest {
    authed(TestRequest::delete().uri(uri))
}

fn upsert_body() -> Value {
    json!({
        "name": "Volume Drop",
        "product": "payments",
        "merchant_id": "acme_store",
        "profile_id": ""
    })
}

#[actix_web::test]
async fn every_route_is_behind_the_guard() {
    for request in [
        TestRequest::post().uri(BASE).set_json(upsert_body()),
        TestRequest::post()
            .uri(&format!("{BASE}/list"))
            .set_json(json!({})),
        TestRequest::post()
            .uri(&format!("{BASE}/update"))
            .set_json(json!({
                "merchant_id": "acme_store",
                "thresholds_min_volume": 50
            })),
        TestRequest::post()
            .uri(&format!("{BASE}/delete"))
            .set_json(json!({
                "name": "Volume Drop",
                "product": "payments"
            })),
        TestRequest::delete().uri(&format!("{BASE}/abc")),
    ] {
        let (status, body) = call(request).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

#[actix_web::test]
async fn an_upsert_without_merchant_id_is_refused() {
    let mut body = upsert_body();
    body.as_object_mut().unwrap().remove("merchant_id");

    let (status, body) = call(post(BASE, body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_upsert_without_profile_id_is_refused() {
    let mut body = upsert_body();
    body.as_object_mut().unwrap().remove("profile_id");

    let (status, body) = call(post(BASE, body)).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn an_unknown_field_is_refused_on_every_write() {
    for uri in [
        BASE.to_owned(),
        format!("{BASE}/update"),
        format!("{BASE}/delete"),
    ] {
        let mut body = upsert_body();
        body.as_object_mut()
            .unwrap()
            .insert("unexpected".to_owned(), json!("x"));

        let (status, body) = call(post(&uri, body)).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri} must refuse it");
        assert_eq!(body["error"]["code"], "IR_04");
    }
}

#[actix_web::test]
async fn update_refuses_bad_input_before_the_database() {
    let (status, body) = call(post(&format!("{BASE}/update"), json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");

    let (status, body) = call(post(
        &format!("{BASE}/update"),
        json!({ "merchant_id": "acme_store" }),
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn delete_refuses_blank_id_before_the_database() {
    let (status, body) = call(post(
        &format!("{BASE}/delete"),
        json!({ "name": "Volume Drop" }),
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");

    let (status, body) = call(delete(&format!("{BASE}/%20"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}
