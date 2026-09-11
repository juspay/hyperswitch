#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::print_stderr
)]

use std::{collections::HashMap, sync::Arc};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_models::observability::schema::{alerts_main, alerts_main_xyne};
use observability::{
    auth::X_INTERNAL_API_KEY,
    domain::notifier::Registry,
    routes::Alerts,
    settings::DatabaseSettings,
    state::{build_database_pool, AppState},
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";

const GENEROUS_CAP: usize = 500;

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

fn state_with(database: &Value, cap: usize) -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
        "instances": { "max_merchants": cap, "max_dimensions": cap }
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

fn instances_uri(channel: &str, announcement: &str) -> String {
    format!("/alerts/instances/{channel}/{announcement}")
}

fn dimensions_uri(announcement: &str) -> String {
    format!("/alerts/dimensions/{announcement}")
}

fn absent_announcement() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[actix_web::test]
async fn every_instance_route_is_behind_the_guard() {
    let announcement = absent_announcement();
    let routes = [
        (TestRequest::get(), instances_uri("slack", &announcement)),
        (
            TestRequest::post().set_json(json!({ "merchants": [] })),
            instances_uri("xyne", &announcement),
        ),
        (TestRequest::get(), dimensions_uri(&announcement)),
        (
            TestRequest::post().set_json(json!({ "dimensions": [] })),
            dimensions_uri(&announcement),
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
async fn an_unreadable_store_is_a_503_and_never_an_empty_list() {
    let announcement = absent_announcement();
    let away = state_with(&unreachable(), GENEROUS_CAP);

    for uri in [
        instances_uri("slack", &announcement),
        dimensions_uri(&announcement),
    ] {
        let (status, body) = call(get(&uri), &away).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{uri}");
        assert_eq!(body["error"]["code"], "HE_01");
        assert!(body.get("status").is_none());
    }
}

#[actix_web::test]
async fn an_unknown_channel_is_a_404_rather_than_a_default() {
    let announcement = absent_announcement();
    let routes = [
        (TestRequest::get(), instances_uri("teams", &announcement)),
        (
            TestRequest::post().set_json(json!({ "merchants": [] })),
            instances_uri("SLACK", &announcement),
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
async fn an_unknown_field_is_a_400_in_our_shape() {
    let (status, body) = call(
        post(
            &instances_uri("slack", &absent_announcement()),
            json!({ "merchants": [{ "merchant_id": "merchant_1234", "sr": 41.5 }] }),
        ),
        &state_with(&unreachable(), GENEROUS_CAP),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}

async fn announce(state: &AppState, channel: &str, thread: Option<&str>) -> String {
    let (status, body) = call(
        post(
            &format!("/alerts/lifecycle/{channel}/announcements"),
            json!({
                "name": "sr_drop",
                "product": "payments",
                "sent": true,
                "ts_slack": thread,
                "critical": true,
                "duration": 900
            }),
        ),
        state,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{body}");
    body["announcement"]["id"].as_str().unwrap().to_owned()
}

async fn forget_announcement(state: &AppState, channel: &str, announcement: &str) {
    let connection = state.database_connection().await.expect("a connection");
    let raw = connection.raw_connection();
    let id = uuid::Uuid::parse_str(announcement).expect("the announcement id should parse");

    match channel {
        "slack" => {
            diesel::delete(alerts_main::table.filter(alerts_main::id.eq(id)))
                .execute_async(raw)
                .await
        }
        "xyne" => {
            diesel::delete(alerts_main_xyne::table.filter(alerts_main_xyne::id.eq(id)))
                .execute_async(raw)
                .await
        }
        other => panic!("no such channel: {other}"),
    }
    .expect("the announcement cleanup should run");
}

fn merchant(id: &str, current: f64, expected: Option<f64>) -> Value {
    json!({
        "merchant_id": id,
        "name": "sr_drop",
        "product": "payments",
        "current_metric": current,
        "expected_metric": expected,
        "priority": "SEV2"
    })
}

#[actix_web::test]
#[ignore]
async fn an_instance_round_trips_without_loss() {
    let state = state();
    let announcement = announce(&state, "slack", Some("1757400000.000100")).await;

    let instance = json!({
        "merchant_id": "merchant_1234",
        "name": "sr_drop",
        "product": "payments",
        "dimensions": { "connector": "stripe" },
        "auxiliary_dimensions": { "region": "eu" },
        "current_metric": 41.5,
        "expected_metric": 92.0,
        "attribution": "connector",
        "max_duration": 2700,
        "start_time": "2026-09-09T10:00:00.000Z",
        "is_visible": true,
        "recovered_ts": null,
        "ts_slack": "1757400000.000100",
        "latest_ts_alert": "2026-09-09T10:45:00.000Z",
        "slack_info": { "channel": "sr_alerts" },
        "communication_info": { "email": "sent" },
        "metadata": { "note": "kept" },
        "metadata_alert_details": { "detail": "kept" },
        "priority": "SEV2",
        "tenant_id": "public"
    });

    let (status, saved) = call(
        post(
            &instances_uri("slack", &announcement),
            json!({ "merchants": [instance.clone()] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["status"], "saved");
    assert_eq!(saved["merchants"], 1);
    assert_eq!(saved["removed"], 0);
    assert!(saved["truncated"].is_null());

    let (status, read) = call(get(&instances_uri("slack", &announcement)), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["status"], "found");
    assert_eq!(read["merchants"].as_array().unwrap().len(), 1);

    let stored = &read["merchants"][0];
    for field in [
        "merchant_id",
        "name",
        "product",
        "dimensions",
        "auxiliary_dimensions",
        "current_metric",
        "expected_metric",
        "attribution",
        "max_duration",
        "start_time",
        "is_visible",
        "recovered_ts",
        "ts_slack",
        "latest_ts_alert",
        "slack_info",
        "communication_info",
        "metadata",
        "metadata_alert_details",
        "priority",
        "tenant_id",
    ] {
        assert_eq!(stored[field], instance[field], "{field} did not survive");
    }

    assert!(stored["id_merchant_table"].is_string());
    assert_eq!(stored["announcement_id"], announcement);
    assert_eq!(stored["ts_alert"], saved["ts_alert"]);
    assert!(stored["last_updated_at"].is_string());

    forget_announcement(&state, "slack", &announcement).await;
}

#[actix_web::test]
#[ignore]
async fn a_dimension_round_trips_without_loss() {
    let state = state();
    let announcement = announce(&state, "slack", Some("1757400000.000200")).await;

    let row = json!({
        "dimension_key": "connector",
        "dimension_value": "stripe",
        "name": "sr_drop",
        "product": "payments",
        "current_metric": 41.5,
        "expected_metric": 92.0,
        "max_duration": 2700,
        "priority": "SEV2"
    });

    let (status, saved) = call(
        post(
            &dimensions_uri(&announcement),
            json!({ "dimensions": [row.clone()] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["dimensions"], 1);
    assert!(saved["truncated"].is_null());

    let (status, read) = call(get(&dimensions_uri(&announcement)), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["status"], "found");

    let stored = &read["dimensions"][0];
    for field in [
        "dimension_key",
        "dimension_value",
        "name",
        "product",
        "current_metric",
        "expected_metric",
        "max_duration",
        "priority",
    ] {
        assert_eq!(stored[field], row[field], "{field} did not survive");
    }
    assert_eq!(stored["announcement_id"], announcement);
    assert_eq!(stored["is_visible"], true);
    assert_eq!(stored["ts_slack"], "1757400000.000200");

    forget_announcement(&state, "slack", &announcement).await;
}

#[actix_web::test]
#[ignore]
async fn removing_an_announcement_takes_its_instances_and_its_breakdown_with_it() {
    let state = state();
    let announcement = announce(&state, "slack", Some("1757400000.000300")).await;

    let (status, _) = call(
        post(
            &instances_uri("slack", &announcement),
            json!({ "merchants": [merchant("merchant_1234", 41.5, Some(92.0))] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = call(
        post(
            &dimensions_uri(&announcement),
            json!({ "dimensions": [{ "dimension_key": "connector", "dimension_value": "stripe" }] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, before) = call(get(&instances_uri("slack", &announcement)), &state).await;
    assert_eq!(before["status"], "found");
    let (_, before) = call(get(&dimensions_uri(&announcement)), &state).await;
    assert_eq!(before["status"], "found");

    forget_announcement(&state, "slack", &announcement).await;

    let (status, after) = call(get(&instances_uri("slack", &announcement)), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        after["status"], "absent",
        "the instances outlived their announcement"
    );
    assert_eq!(after["merchants"], json!([]));

    let (status, after) = call(get(&dimensions_uri(&announcement)), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        after["status"], "absent",
        "the breakdown outlived its announcement"
    );
    assert_eq!(after["dimensions"], json!([]));
}

#[actix_web::test]
#[ignore]
async fn a_write_against_a_missing_announcement_is_refused_and_nothing_is_written() {
    let state = state();
    let missing = absent_announcement();

    for (uri, body) in [
        (
            instances_uri("slack", &missing),
            json!({ "merchants": [merchant("merchant_1234", 41.5, Some(92.0))] }),
        ),
        (
            dimensions_uri(&missing),
            json!({ "dimensions": [{ "dimension_key": "connector", "dimension_value": "stripe" }] }),
        ),
    ] {
        let (status, answer) = call(post(&uri, body), &state).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(answer["error"]["code"], "IR_12");
    }

    let (_, read) = call(get(&instances_uri("slack", &missing)), &state).await;
    assert_eq!(read["status"], "absent");
}

#[actix_web::test]
#[ignore]
async fn a_second_write_replaces_the_first_rather_than_doubling_it() {
    let state = state();
    let announcement = announce(&state, "slack", Some("1757400000.000400")).await;
    let body = json!({
        "merchants": [
            merchant("merchant_1234", 41.5, Some(92.0)),
            merchant("merchant_5678", 10.0, Some(90.0))
        ]
    });

    let (status, first) = call(
        post(&instances_uri("slack", &announcement), body.clone()),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["merchants"], 2);
    assert_eq!(first["removed"], 0);

    let (status, again) = call(post(&instances_uri("slack", &announcement), body), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(again["merchants"], 2);
    assert_eq!(
        again["removed"], 2,
        "the rerun added to the table instead of replacing it"
    );

    let (_, read) = call(get(&instances_uri("slack", &announcement)), &state).await;
    assert_eq!(read["merchants"].as_array().unwrap().len(), 2);

    let (status, cleared) = call(
        post(
            &instances_uri("slack", &announcement),
            json!({ "merchants": [] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cleared["merchants"], 0);
    assert_eq!(cleared["removed"], 2);

    let (_, read) = call(get(&instances_uri("slack", &announcement)), &state).await;
    assert_eq!(read["status"], "absent");

    forget_announcement(&state, "slack", &announcement).await;
}

#[actix_web::test]
#[ignore]
async fn the_two_channels_hold_their_own_instances() {
    let state = state();
    let slack = announce(&state, "slack", Some("1757400000.000500")).await;
    let xyne = announce(&state, "xyne", Some("1757400000.000600")).await;

    for (channel, announcement) in [("slack", &slack), ("xyne", &xyne)] {
        let (status, saved) = call(
            post(
                &instances_uri(channel, announcement),
                json!({ "merchants": [merchant(channel, 41.5, Some(92.0))] }),
            ),
            &state,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{saved}");
    }

    for (channel, announcement) in [("slack", &slack), ("xyne", &xyne)] {
        let (_, read) = call(get(&instances_uri(channel, announcement)), &state).await;
        assert_eq!(read["merchants"].as_array().unwrap().len(), 1);
        assert_eq!(read["merchants"][0]["merchant_id"], channel);
    }

    let (status, body) = call(
        post(
            &instances_uri("xyne", &slack),
            json!({ "merchants": [merchant("merchant_1234", 41.5, Some(92.0))] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_12");

    forget_announcement(&state, "slack", &slack).await;
    forget_announcement(&state, "xyne", &xyne).await;
}

#[actix_web::test]
#[ignore]
async fn a_breakdown_over_the_cap_is_stored_cut_down_and_says_so() {
    let capped = state_with(&local(), 2);
    let announcement = announce(&capped, "slack", Some("1757400000.000700")).await;

    let (status, saved) = call(
        post(
            &dimensions_uri(&announcement),
            json!({
                "dimensions": [
                    { "dimension_key": "connector", "dimension_value": "barely", "current_metric": 89.0, "expected_metric": 90.0 },
                    { "dimension_key": "connector", "dimension_value": "badly", "current_metric": 5.0, "expected_metric": 90.0 },
                    { "dimension_key": "connector", "dimension_value": "zero_volume", "current_metric": 0.0 },
                    { "dimension_key": "connector", "dimension_value": "somewhat", "current_metric": 60.0, "expected_metric": 90.0 }
                ]
            }),
        ),
        &capped,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["dimensions"], 2);
    assert_eq!(saved["truncated"]["received"], 4);
    assert_eq!(saved["truncated"]["stored"], 2);
    assert_eq!(saved["truncated"]["dropped"], 2);

    let (_, read) = call(get(&dimensions_uri(&announcement)), &capped).await;
    let kept: Vec<&str> = read["dimensions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["dimension_value"].as_str().unwrap())
        .collect();

    assert_eq!(kept.len(), 2);
    assert!(
        kept.contains(&"zero_volume"),
        "the absolute was dropped: {kept:?}"
    );
    assert!(
        kept.contains(&"badly"),
        "the widest gap was dropped: {kept:?}"
    );

    for row in read["dimensions"].as_array().unwrap() {
        let marker = &row["metadata_alert_details"]["truncated_by_impact"];
        assert_eq!(marker["received"], 4, "{row}");
        assert_eq!(marker["stored"], 2);
        assert_eq!(marker["dropped"], 2);
    }

    forget_announcement(&capped, "slack", &announcement).await;
}

#[actix_web::test]
#[ignore]
async fn the_defaults_the_columns_lost_are_supplied_by_the_service() {
    let state = state();
    let announcement = announce(&state, "slack", Some("1757400000.000800")).await;

    let (status, saved) = call(
        post(
            &instances_uri("slack", &announcement),
            json!({
                "merchants": [
                    { "merchant_id": "merchant_1" },
                    { "merchant_id": "merchant_2" },
                    { "merchant_id": "merchant_3" }
                ]
            }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["merchants"], 3);

    let (_, read) = call(get(&instances_uri("slack", &announcement)), &state).await;
    let rows = read["merchants"].as_array().unwrap();
    assert_eq!(rows.len(), 3);

    let mut ids: Vec<&str> = rows
        .iter()
        .map(|row| row["id_merchant_table"].as_str().unwrap())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 3, "two rows were written under one primary key");

    for row in rows {
        assert!(row["id_merchant_table"].is_string());
        assert_eq!(row["is_visible"], true, "a row was stored invisible");
        assert!(
            row["ts_alert"].is_string(),
            "a row was stored without a timestamp"
        );
        assert!(row["last_updated_at"].is_string());
        assert_eq!(row["ts_slack"], "1757400000.000800");
        assert!(row["current_metric"].is_null());
        assert!(row["expected_metric"].is_null());
    }

    forget_announcement(&state, "slack", &announcement).await;
}

#[actix_web::test]
#[ignore]
async fn an_instance_recorded_before_its_announcement_reached_a_channel_has_no_thread() {
    let state = state();
    let announcement = announce(&state, "slack", None).await;

    let (status, saved) = call(
        post(
            &instances_uri("slack", &announcement),
            json!({ "merchants": [{ "merchant_id": "merchant_1234" }] }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");

    let (_, read) = call(get(&instances_uri("slack", &announcement)), &state).await;
    assert!(read["merchants"][0]["ts_slack"].is_null());

    forget_announcement(&state, "slack", &announcement).await;
}
