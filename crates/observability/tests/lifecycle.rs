#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::print_stderr
)]

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::QueryDsl;
use diesel_models::observability::schema::{
    alerts_intermediate, alerts_intermediate_xyne, alerts_main, alerts_main_xyne,
};
use observability::{
    auth::X_INTERNAL_API_KEY,
    domain::notifier::Registry,
    routes::Alerts,
    settings::DatabaseSettings,
    state::{build_database_pool, AppState},
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";

const GENEROUS_CAP: usize = 5_000;

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn unreachable() -> Value {
    json!({
        "host": "127.0.0.1",
        "port": 1,
        "dbname": "unused",
        "username": "unused",
        "password": "unused",
        "connection_timeout": 1
    })
}

fn local() -> Value {
    json!({
        "host": "127.0.0.1",
        "port": 5432,
        "dbname": "observability",
        "username": "db_user",
        "password": "db_pass",
        "connection_timeout": 2
    })
}

fn state_with(database: &Value, max_alerts: usize) -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
        "lifecycle": { "max_alerts": max_alerts }
    }))
    .expect("the test configuration should deserialize");

    let database: DatabaseSettings = serde_json::from_value(database.clone())
        .expect("the test database configuration should deserialize");

    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::new(HashMap::new())),
        email: Arc::new(Registry::new(HashMap::new())),
        database: build_database_pool(&database).expect("an unchecked pool should build"),
    }
}

fn state() -> AppState {
    state_with(&local(), GENEROUS_CAP)
}

async fn call(request: TestRequest, state: &AppState) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state.clone()))).await;
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

fn state_uri(channel: &str) -> String {
    format!("/alerts/lifecycle/{channel}/state")
}

fn announcements_uri(channel: &str) -> String {
    format!("/alerts/lifecycle/{channel}/announcements")
}

fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    format!(
        "{prefix}_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[actix_web::test]
async fn every_lifecycle_route_is_behind_the_guard() {
    let routes = [
        (TestRequest::get(), state_uri("slack")),
        (
            TestRequest::post().set_json(json!({ "alerts": [] })),
            state_uri("slack"),
        ),
        (
            TestRequest::post().set_json(json!({ "sent": true })),
            announcements_uri("xyne"),
        ),
    ];

    for (request, uri) in routes {
        let (status, body) =
            call(request.uri(&uri), &state_with(&unreachable(), GENEROUS_CAP)).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} must be guarded");
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

#[actix_web::test]
async fn an_unreadable_state_is_a_503_and_never_an_empty_list() {
    let (status, body) = call(
        get(&state_uri("slack")),
        &state_with(&unreachable(), GENEROUS_CAP),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "HE_01");
    assert!(body.get("alerts").is_none());
    assert!(body.get("status").is_none());
}

#[actix_web::test]
async fn an_unknown_channel_is_a_404_rather_than_a_default() {
    let routes = [
        (TestRequest::get(), state_uri("teams")),
        (
            TestRequest::post().set_json(json!({ "alerts": [] })),
            state_uri("SLACK"),
        ),
        (
            TestRequest::post().set_json(json!({ "sent": true })),
            announcements_uri("Xyne"),
        ),
    ];

    for (request, uri) in routes {
        let (status, body) = call(
            request
                .uri(&uri)
                .insert_header((X_INTERNAL_API_KEY, API_KEY)),
            &state_with(&unreachable(), GENEROUS_CAP),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(body["error"]["code"], "IR_09");
    }
}

#[actix_web::test]
async fn a_write_over_the_alert_cap_is_refused_before_it_is_stored() {
    let alerts: Vec<Value> = (0..3).map(|_| json!({ "name": "sr_drop" })).collect();

    let (status, body) = call(
        post(&state_uri("slack"), json!({ "alerts": alerts })),
        &state_with(&unreachable(), 2),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_10");
    assert!(!body.to_string().contains('3'));
}

#[actix_web::test]
async fn an_unknown_field_in_a_state_write_is_a_400_in_our_shape() {
    let (status, body) = call(
        post(
            &state_uri("slack"),
            json!({ "alerts": [{ "name": "sr_drop", "runs": 4 }] }),
        ),
        &state_with(&unreachable(), GENEROUS_CAP),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}

async fn clear(state: &AppState, channel: &str) {
    let connection = state.database_connection().await.expect("a connection");
    let raw = connection.raw_connection();

    match channel {
        "slack" => {
            diesel::delete(alerts_intermediate::table)
                .execute_async(raw)
                .await
                .expect("the state cleanup should run");
            diesel::delete(alerts_main::table)
                .execute_async(raw)
                .await
                .expect("the announcement cleanup should run");
        }
        "xyne" => {
            diesel::delete(alerts_intermediate_xyne::table)
                .execute_async(raw)
                .await
                .expect("the state cleanup should run");
            diesel::delete(alerts_main_xyne::table)
                .execute_async(raw)
                .await
                .expect("the announcement cleanup should run");
        }
        other => panic!("no such channel: {other}"),
    }
}

async fn announcement_count(state: &AppState, channel: &str) -> i64 {
    let connection = state.database_connection().await.expect("a connection");
    let raw = connection.raw_connection();

    match channel {
        "slack" => alerts_main::table.count().get_result_async(raw).await,
        "xyne" => alerts_main_xyne::table.count().get_result_async(raw).await,
        other => panic!("no such channel: {other}"),
    }
    .expect("the count should run")
}

async fn announce(state: &AppState, channel: &str, name: &str, sent: bool, thread: &str) -> String {
    let (status, body) = call(
        post(
            &announcements_uri(channel),
            json!({
                "name": name,
                "product": "payments",
                "sent": sent,
                "ts_slack": thread,
                "critical": true,
                "duration": 900,
                "dimensions": { "merchant_id": "merchant_1234" }
            }),
        ),
        state,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    body["announcement"]["id"].as_str().unwrap().to_owned()
}

async fn watermark(state: &AppState, channel: &str) -> Value {
    let (status, body) = call(get(&state_uri(channel)), state).await;

    assert_eq!(status, StatusCode::OK);
    body["last_updated_at"].clone()
}

#[actix_web::test]
#[ignore]
async fn a_full_read_and_write_round_trip_without_loss() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "slack").await;

    let announcement = announce(&state, "slack", &name, true, "1757400000.000100").await;
    let alert = json!({
        "announcement_id": announcement,
        "name": name,
        "product": "payments",
        "dimensions": { "merchant_id": "merchant_1234", "connector": "stripe" },
        "ts_slack": "1757400000.000100",
        "ts_alert": "2026-09-09T10:00:00.000Z",
        "latest_ts_alert": "2026-09-09T10:45:00.000Z",
        "max_duration": 2700,
        "other_metrics": { "current_metric": 41.5, "expected_metric": 92.0 },
        "metadata": { "note": "kept" },
        "metadata_alert_details": { "detail": "kept" },
        "rca_metadata": { "cause": "unknown" },
        "group_id": "sr_drop|merchant_1234|stripe",
        "priority": "SEV2",
        "recovered_ts": null
    });

    let (status, saved) = call(
        post(
            &state_uri("slack"),
            json!({ "expected_last_updated_at": null, "alerts": [alert.clone()] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["status"], "saved");
    assert_eq!(saved["alerts"], 1);

    let (status, read) = call(get(&state_uri("slack")), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["status"], "found");
    assert_eq!(read["alerts"].as_array().unwrap().len(), 1);

    let stored = &read["alerts"][0];
    for field in [
        "announcement_id",
        "name",
        "product",
        "dimensions",
        "ts_slack",
        "ts_alert",
        "latest_ts_alert",
        "max_duration",
        "other_metrics",
        "metadata",
        "metadata_alert_details",
        "rca_metadata",
        "group_id",
        "priority",
        "recovered_ts",
    ] {
        assert_eq!(stored[field], alert[field], "{field} did not survive");
    }

    assert!(stored["id_intermediate"].is_string());
    assert_eq!(stored["last_updated_at"], saved["last_updated_at"]);

    clear(&state, "slack").await;
}

#[actix_web::test]
#[ignore]
async fn empty_state_is_a_200_that_says_so() {
    let _serial = SERIAL.lock().await;
    let state = state();
    clear(&state, "slack").await;

    let (status, body) = call(get(&state_uri("slack")), &state).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "absent");
    assert_eq!(body["alerts"], json!([]));
    assert!(body["last_updated_at"].is_null());
}

#[actix_web::test]
#[ignore]
async fn an_overlapping_write_cannot_put_back_older_state() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "slack").await;

    let read_by_both = watermark(&state, "slack").await;

    let (status, _) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": read_by_both,
                "alerts": [{ "name": name, "group_id": "still_firing" }]
            }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "the first run should land");

    let (status, body) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": read_by_both,
                "alerts": [{ "name": name, "group_id": "stale" }]
            }),
        ),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "IR_11");

    let (_, read) = call(get(&state_uri("slack")), &state).await;
    assert_eq!(read["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(read["alerts"][0]["group_id"], "still_firing");

    clear(&state, "slack").await;
}

#[actix_web::test]
#[ignore]
async fn a_write_waits_for_the_channel_lock() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "slack").await;

    let holder = state.database_connection().await.expect("a connection");
    diesel::sql_query("SELECT pg_advisory_lock(23404, 1)")
        .execute_async(holder.raw_connection())
        .await
        .expect("the lock should be taken");

    let writing = state.clone();
    let group = name.clone();
    let mut writer = actix_web::rt::spawn(async move {
        call(
            post(
                &state_uri("slack"),
                json!({
                    "expected_last_updated_at": null,
                    "alerts": [{ "name": group, "group_id": "waited" }]
                }),
            ),
            &writing,
        )
        .await
    });

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(750), &mut writer)
            .await
            .is_err(),
        "a whole-state write went through while the channel lock was held"
    );

    diesel::sql_query("SELECT pg_advisory_unlock(23404, 1)")
        .execute_async(holder.raw_connection())
        .await
        .expect("the lock should be released");

    let (status, body) = writer.await.expect("the write task should finish");
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, read) = call(get(&state_uri("slack")), &state).await;
    assert_eq!(read["alerts"][0]["group_id"], "waited");

    clear(&state, "slack").await;
}

#[actix_web::test]
#[ignore]
async fn an_announcement_records_whether_it_was_delivered_and_to_which_thread() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "xyne").await;

    let (status, body) = call(
        post(
            &announcements_uri("xyne"),
            json!({ "name": name, "sent": false, "ts_slack": null }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["announcement"]["sent"], false);
    assert!(body["announcement"]["ts_slack"].is_null());
    assert!(body["announcement"]["ts_alert"].is_string());

    let (_, delivered) = call(
        post(
            &announcements_uri("xyne"),
            json!({ "name": name, "sent": true, "ts_slack": "1757400000.000200" }),
        ),
        &state,
    )
    .await;
    assert_eq!(delivered["announcement"]["sent"], true);
    assert_eq!(delivered["announcement"]["ts_slack"], "1757400000.000200");

    assert_ne!(
        delivered["announcement"]["id"], body["announcement"]["id"],
        "an announcement replaced the one before it"
    );
    assert_eq!(announcement_count(&state, "xyne").await, 2);

    clear(&state, "xyne").await;
}

#[actix_web::test]
#[ignore]
async fn a_state_write_never_removes_an_announcement() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "slack").await;

    let first = announce(&state, "slack", &name, true, "1757400000.000300").await;
    let second = announce(&state, "slack", &name, true, "1757400000.000400").await;
    assert_eq!(announcement_count(&state, "slack").await, 2);

    let (status, _) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": null,
                "alerts": [
                    { "announcement_id": first, "name": name, "group_id": "one" },
                    { "announcement_id": second, "name": name, "group_id": "two" }
                ]
            }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, read) = call(get(&state_uri("slack")), &state).await;
    let surviving = read["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alert| alert["group_id"] == "two")
        .expect("the second episode should be stored")
        .clone();

    let (status, saved) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": read["last_updated_at"],
                "alerts": [{
                    "id_intermediate": surviving["id_intermediate"],
                    "announcement_id": second,
                    "name": name,
                    "group_id": "two"
                }]
            }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["removed"], 1);
    assert_eq!(saved["alerts"], 1);

    assert_eq!(announcement_count(&state, "slack").await, 2);

    let expected = watermark(&state, "slack").await;
    let (status, _) = call(
        post(
            &state_uri("slack"),
            json!({ "expected_last_updated_at": expected, "alerts": [] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(announcement_count(&state, "slack").await, 2);

    let (_, read) = call(get(&state_uri("slack")), &state).await;
    assert_eq!(read["status"], "absent");

    clear(&state, "slack").await;
}

#[actix_web::test]
#[ignore]
async fn a_row_referencing_a_missing_announcement_is_refused_and_nothing_is_written() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "slack").await;

    let real = announce(&state, "slack", &name, true, "1757400000.000500").await;
    let missing = uuid::Uuid::now_v7().to_string();

    let (status, body) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": null,
                "alerts": [
                    { "announcement_id": real, "name": name, "group_id": "good" },
                    { "announcement_id": missing, "name": name, "group_id": "dangling" }
                ]
            }),
        ),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_12");

    let (_, read) = call(get(&state_uri("slack")), &state).await;
    assert_eq!(read["status"], "absent");
    assert_eq!(read["alerts"], json!([]));

    clear(&state, "slack").await;
}

#[actix_web::test]
#[ignore]
async fn the_two_channels_hold_their_own_state() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "slack").await;
    clear(&state, "xyne").await;

    let (status, _) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": null,
                "alerts": [{ "name": name, "group_id": "slack_only" }]
            }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, slack) = call(get(&state_uri("slack")), &state).await;
    assert_eq!(slack["status"], "found");

    let (_, xyne) = call(get(&state_uri("xyne")), &state).await;
    assert_eq!(xyne["status"], "absent");
    assert_eq!(xyne["alerts"], json!([]));

    clear(&state, "slack").await;
}

#[actix_web::test]
#[ignore]
async fn a_row_written_without_an_id_is_given_one_that_survives_the_next_write() {
    let _serial = SERIAL.lock().await;
    let state = state();
    let name = unique("sr_drop");
    clear(&state, "slack").await;

    let (status, _) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": null,
                "alerts": [{ "name": name, "ts_alert": "2026-09-09T10:00:00.000Z" }]
            }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, read) = call(get(&state_uri("slack")), &state).await;
    let minted = read["alerts"][0]["id_intermediate"]
        .as_str()
        .unwrap()
        .to_owned();

    let (status, saved) = call(
        post(
            &state_uri("slack"),
            json!({
                "expected_last_updated_at": read["last_updated_at"],
                "alerts": [{
                    "id_intermediate": minted,
                    "name": name,
                    "ts_alert": "2026-09-09T10:00:00.000Z",
                    "latest_ts_alert": "2026-09-09T11:00:00.000Z"
                }]
            }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(
        saved["removed"], 0,
        "echoing the id should not replace the row"
    );

    let (_, read) = call(get(&state_uri("slack")), &state).await;
    assert_eq!(read["alerts"].as_array().unwrap().len(), 1);
    assert_eq!(read["alerts"][0]["id_intermediate"], minted);
    assert_eq!(read["alerts"][0]["ts_alert"], "2026-09-09T10:00:00.000Z");
    assert_eq!(
        read["alerts"][0]["latest_ts_alert"],
        "2026-09-09T11:00:00.000Z"
    );

    clear(&state, "slack").await;
}
