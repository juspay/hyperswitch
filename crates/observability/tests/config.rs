#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stderr
)]

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_models::observability::schema::{alerts_dicts, notification_reads};
use observability::{
    alert_manager::types::X_USER_NAME,
    auth::X_INTERNAL_API_KEY,
    domain::notifier::Registry,
    routes::Alerts,
    settings::DatabaseSettings,
    state::{build_database_pool, AppState},
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";
const PRODUCT: &str = "payments";

const DEFAULT_USERNAME: &str = "reliability_team";

const GENEROUS_CAP: usize = 1024 * 1024;

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

fn state_with(database: &Value) -> AppState {
    let conf = serde_json::from_value(json!({ "auth": { "internal_api_key": API_KEY } }))
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

async fn state() -> AppState {
    state_with_cap(GENEROUS_CAP).await
}

async fn state_with_cap(max_entry_bytes: usize) -> AppState {
    let conf = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
        "database": {
            "host": "127.0.0.1",
            "port": 5432,
            "dbname": "observability",
            "username": "db_user",
            "password": "db_pass"
        },
        "dictionary": { "max_entry_bytes": max_entry_bytes }
    }))
    .expect("the test configuration should deserialize");

    let database = build_database_pool(
        &serde_json::from_value(json!({
            "host": "127.0.0.1",
            "port": 5432,
            "dbname": "observability",
            "username": "db_user",
            "password": "db_pass"
        }))
        .expect("the test database configuration should deserialize"),
    )
    .expect("an unchecked pool should build without connecting");

    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::default()),
        email: Arc::new(Registry::default()),
        database,
    }
}

async fn call(request: TestRequest) -> (StatusCode, Value) {
    call_with_state(request, &state().await).await
}

async fn call_with_state(request: TestRequest, state: &AppState) -> (StatusCode, Value) {
    let (status, body) = call_raw(request, state).await;

    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

async fn call_raw(request: TestRequest, state: &AppState) -> (StatusCode, String) {
    let app = test::init_service(App::new().service(Alerts::server(state.clone()))).await;
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = test::read_body(response).await;

    (status, String::from_utf8(body.to_vec()).unwrap())
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

fn delete(uri: &str) -> TestRequest {
    TestRequest::delete()
        .uri(uri)
        .insert_header((X_INTERNAL_API_KEY, API_KEY))
}

fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!(
        "{prefix}_{nanos}_{}",
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

#[actix_web::test]
async fn every_config_route_is_behind_the_guard() {
    const ID: &str = "/alerts/config/definitions/0189d0a0-0000-7000-8000-000000000000";

    let routes = [
        (TestRequest::get(), "/alerts/config/definitions".to_owned()),
        (
            TestRequest::post().set_json(definition_body("sr_drop", true)),
            "/alerts/config/definitions".to_owned(),
        ),
        (TestRequest::get(), ID.to_owned()),
        (
            TestRequest::post().set_json(json!({ "is_enabled": false })),
            ID.to_owned(),
        ),
        (TestRequest::get(), "/alerts/config/enablement".to_owned()),
        (
            TestRequest::get(),
            "/alerts/config/enablement/sr_drop/payments".to_owned(),
        ),
        (
            TestRequest::post().set_json(json!({ "is_enabled": true })),
            "/alerts/config/enablement/sr_drop/payments".to_owned(),
        ),
    ];

    for (request, uri) in routes {
        let (status, body) = call_with_state(request.uri(&uri), &state_with(&unreachable())).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} must be guarded");
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

#[actix_web::test]
async fn an_unreachable_database_is_a_503_and_never_an_empty_list() {
    for uri in ["/alerts/config/definitions", "/alerts/config/enablement"] {
        let (status, body) = call_with_state(get(uri), &state_with(&unreachable())).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{uri}");
        assert_eq!(body["error"]["code"], "HE_01");
        assert!(body.get("count").is_none());
    }
}

#[actix_web::test]
async fn an_unreachable_database_does_not_describe_itself_to_the_caller() {
    let (_, body) = call_with_state(
        get("/alerts/config/definitions"),
        &state_with(&unreachable()),
    )
    .await;
    let rendered = body.to_string();

    assert!(!rendered.contains("unused"));
    assert!(!rendered.contains("127.0.0.1"));
}

#[actix_web::test]
async fn a_definition_missing_a_required_field_is_a_400_in_our_shape() {
    let (status, body) = call_with_state(
        post(
            "/alerts/config/definitions",
            json!({ "name": "sr_drop", "product": PRODUCT }),
        ),
        &state_with(&unreachable()),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}

#[actix_web::test]
async fn a_definition_id_that_is_not_a_uuid_does_not_route() {
    let (status, _) = call_with_state(
        get("/alerts/config/definitions/sr_drop"),
        &state_with(&unreachable()),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn an_entry_over_the_configured_cap_is_refused_before_it_is_stored() {
    let state = state_with_cap(64).await;
    let oversized = format!("\"{}\"", "x".repeat(128));

    let (status, body) = call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": "mappers", "key_": "oversized", "values_": oversized }),
        ),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_08");
}

#[actix_web::test]
async fn an_oversized_entry_is_not_echoed_back() {
    let state = state_with_cap(64).await;
    let secret = "merchant_1234_conversion_map";

    let (_, body) = call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": "mappers", "key_": "oversized", "values_": format!("\"{secret}{}\"", "x".repeat(128)) }),
        ),
        &state,
    )
    .await;

    assert!(!body.to_string().contains(secret));
}

#[actix_web::test]
async fn an_entry_with_no_name_or_no_key_is_refused() {
    for body in [
        json!({ "name": "   ", "key_": "channels" }),
        json!({ "name": "mappers", "key_": "" }),
    ] {
        let (status, response) = call(post("/alerts/config/dictionary", body.clone())).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} was accepted");
        assert_eq!(response["error"]["code"], "IR_04");
    }
}

#[actix_web::test]
async fn an_entry_wider_than_its_column_is_refused_rather_than_left_to_the_database() {
    let (status, body) = call(post(
        "/alerts/config/dictionary",
        json!({ "name": "n".repeat(65), "key_": "channels" }),
    ))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn a_save_rejects_unknown_fields() {
    let (status, body) = call(post(
        "/alerts/config/dictionary",
        json!({ "name": "mappers", "key_": "channels", "value_": "[]" }),
    ))
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

#[actix_web::test]
async fn every_state_route_is_behind_the_guard() {
    for request in [
        TestRequest::get().uri("/alerts/config/dictionary"),
        TestRequest::post()
            .uri("/alerts/config/dictionary")
            .set_json(json!({ "name": "mappers", "key_": "channels" })),
        TestRequest::get().uri("/alerts/config/dictionary/mappers/channels"),
        TestRequest::delete().uri("/alerts/config/dictionary/mappers/channels"),
        TestRequest::get().uri("/alerts/config/notifications/read"),
        TestRequest::post().uri("/alerts/config/notifications/read"),
    ] {
        let (status, body) = call(request).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

#[actix_web::test]
async fn a_user_header_that_is_not_utf8_is_refused() {
    let (status, body) = call(
        TestRequest::get()
            .uri("/alerts/config/notifications/read")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .insert_header((
                X_USER_NAME,
                actix_web::http::header::HeaderValue::from_bytes(&[0xff, 0xfe]).unwrap(),
            )),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
}

async fn database_state() -> Option<AppState> {
    let url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://db_user:db_pass@127.0.0.1:5432/observability".to_owned());
    let (credentials, location) = url.trim_start_matches("postgres://").split_once('@')?;
    let (username, password) = credentials.split_once(':')?;
    let (host, rest) = location.split_once(':')?;
    let (port, dbname) = rest.split_once('/')?;

    let state = state_with(&json!({
        "host": host,
        "port": port.parse::<u16>().ok()?,
        "dbname": dbname,
        "username": username,
        "password": password,
        "connection_timeout": 2
    }));

    let probe = state
        .database
        .get()
        .await
        .map(|_| ())
        .map_err(|error| error.to_string());

    match probe {
        Ok(()) => Some(state),
        Err(error) => {
            eprintln!(
                "skipping: no observability database at {host}:{port}/{dbname} ({error}). Set \
                 OBSERVABILITY_TEST_DATABASE_URL to run these."
            );
            None
        }
    }
}

fn unique_name(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    format!(
        "{prefix}_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

async fn forget(state: &AppState, name: &str) {
    let connection = state.database_connection().await.expect("a connection");

    for statement in [
        format!("DELETE FROM merchants_alert_external_config WHERE name = '{name}'"),
        format!("DELETE FROM alerts_info WHERE name = '{name}'"),
    ] {
        diesel::sql_query(statement)
            .execute_async(connection.raw_connection())
            .await
            .expect("the cleanup statement should run");
    }
}

fn definition_body(name: &str, is_enabled: bool) -> Value {
    json!({
        "name": name,
        "product": PRODUCT,
        "is_enabled": is_enabled,
        "author": "reliability_team",
        "period": 15,
        "blacklist": [{
            "merchant_id": "merchant_1234",
            "profile_id": "pro_abc",
            "reason": "dead test merchant",
            "created_by": "reliability_team"
        }],
        "snooze": [{
            "merchant_id": "merchant_5678",
            "profile_id": "",
            "connector": "stripe",
            "payment_method": null,
            "starts_at": "2026-12-01T00:00:00.000Z",
            "ends_at": "2026-12-31T23:59:00.000Z",
            "created_by": "reliability_team"
        }],
        "thresholds": [{
            "merchant_id": "merchant_1234",
            "profile_id": "",
            "min_volume": 0.0,
            "min_impacted_volume": null,
            "tolerance": 2.5,
            "diff_threshold": null
        }],
        "metadata": { "owner": "reliability" }
    })
}

macro_rules! with_database {
    ($state:ident) => {
        let Some($state) = database_state().await else {
            return;
        };
    };
}

#[actix_web::test]
async fn a_definition_can_be_created_read_and_listed() {
    with_database!(state);
    let name = unique_name("cfg_crud");

    let (status, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");

    let id = created["id"]
        .as_str()
        .expect("an id was assigned")
        .to_owned();
    assert_eq!(created["name"], name);
    assert_eq!(created["is_enabled"], true);
    assert_eq!(created["blacklist"][0]["merchant_id"], "merchant_1234");
    assert_eq!(created["snooze"][0]["ends_at"], "2026-12-31T23:59:00.000Z");
    assert_eq!(created["thresholds"][0]["min_volume"], 0.0);
    assert_eq!(created["metadata"]["owner"], "reliability");

    let (status, read) =
        call_with_state(get(&format!("/alerts/config/definitions/{id}")), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read, created);

    let (status, listed) = call_with_state(get("/alerts/config/definitions"), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        listed["count"].as_u64().unwrap(),
        u64::try_from(listed["definitions"].as_array().unwrap().len()).unwrap()
    );
    assert!(listed["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|definition| definition["id"] == created["id"]));

    forget(&state, &name).await;
}

#[actix_web::test]
async fn the_json_columns_survive_a_round_trip_through_the_database() {
    with_database!(state);
    let name = unique_name("cfg_json");

    let sent = definition_body(&name, true);
    let (_, created) =
        call_with_state(post("/alerts/config/definitions", sent.clone()), &state).await;

    for column in ["blacklist", "snooze", "thresholds", "metadata"] {
        assert_eq!(created[column], sent[column], "{column} did not round trip");
    }

    forget(&state, &name).await;
}

#[actix_web::test]
async fn an_entry_written_without_its_optional_fields_comes_back_with_their_defaults() {
    with_database!(state);
    let name = unique_name("cfg_defaults");

    let (_, created) = call_with_state(
        post(
            "/alerts/config/definitions",
            json!({
                "name": name,
                "product": PRODUCT,
                "is_enabled": true,
                "author": "reliability_team",
                "blacklist": [{ "merchant_id": "merchant_1234" }]
            }),
        ),
        &state,
    )
    .await;

    assert_eq!(
        created["blacklist"][0],
        json!({
            "merchant_id": "merchant_1234",
            "profile_id": "",
            "reason": "",
            "created_by": null
        })
    );

    forget(&state, &name).await;
}

#[actix_web::test]
async fn an_update_leaves_the_columns_it_did_not_mention_alone() {
    with_database!(state);
    let name = unique_name("cfg_partial");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        &state,
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();

    let (status, updated) = call_with_state(
        post(
            &format!("/alerts/config/definitions/{id}"),
            json!({ "thresholds": [{ "merchant_id": "merchant_9999", "tolerance": 5.0 }] }),
        ),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(updated["thresholds"][0]["merchant_id"], "merchant_9999");
    assert_eq!(updated["blacklist"], created["blacklist"]);
    assert_eq!(updated["snooze"], created["snooze"]);
    assert_eq!(updated["period"], 15);
    assert_ne!(updated["last_updated_at"], Value::Null);

    forget(&state, &name).await;
}

#[actix_web::test]
async fn an_explicit_null_clears_a_column_that_an_absent_field_would_have_kept() {
    with_database!(state);
    let name = unique_name("cfg_null");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        &state,
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();

    let (_, kept) = call_with_state(
        post(&format!("/alerts/config/definitions/{id}"), json!({})),
        &state,
    )
    .await;
    assert_eq!(kept["period"], 15);

    let (_, cleared) = call_with_state(
        post(
            &format!("/alerts/config/definitions/{id}"),
            json!({ "period": null, "blacklist": null }),
        ),
        &state,
    )
    .await;
    assert_eq!(cleared["period"], Value::Null);
    assert_eq!(cleared["blacklist"], json!([]));

    forget(&state, &name).await;
}

#[actix_web::test]
async fn an_alert_is_turned_off_and_on_again_through_is_enabled() {
    with_database!(state);
    let name = unique_name("cfg_switch");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        &state,
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();

    for expected in [false, true] {
        let (status, updated) = call_with_state(
            post(
                &format!("/alerts/config/definitions/{id}"),
                json!({ "is_enabled": expected }),
            ),
            &state,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["is_enabled"], expected);
    }

    forget(&state, &name).await;
}

#[actix_web::test]
async fn a_second_definition_with_the_same_name_and_product_is_refused() {
    with_database!(state);
    let name = unique_name("cfg_dup");

    let (first, _) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        &state,
    )
    .await;
    let (second, body) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, false)),
        &state,
    )
    .await;

    assert_eq!(first, StatusCode::OK);
    assert_eq!(second, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_05");

    forget(&state, &name).await;
}

#[actix_web::test]
async fn the_reserved_all_definition_carries_suppression_for_every_detector() {
    with_database!(state);
    let product = unique_name("cfg_all");

    let (status, created) = call_with_state(
        post(
            "/alerts/config/definitions",
            json!({
                "name": "all",
                "product": product,
                "is_enabled": true,
                "author": "reliability_team",
                "blacklist": [{ "merchant_id": "merchant_1234", "reason": "muted everywhere" }]
            }),
        ),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(created["name"], "all");
    assert_eq!(created["blacklist"][0]["merchant_id"], "merchant_1234");

    let connection = state.database_connection().await.expect("a connection");
    diesel::sql_query(format!(
        "DELETE FROM alerts_info WHERE product = '{product}'"
    ))
    .execute_async(connection.raw_connection())
    .await
    .expect("the cleanup statement should run");
}

#[actix_web::test]
async fn an_unknown_definition_id_is_a_404() {
    with_database!(state);

    let (status, body) = call_with_state(
        get("/alerts/config/definitions/0189d0a0-0000-7000-8000-000000000000"),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "IR_03");
}

#[actix_web::test]
async fn a_repeated_enablement_upsert_updates_rather_than_duplicating() {
    with_database!(state);
    let name = unique_name("cfg_enable");

    let (_, _) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        &state,
    )
    .await;

    let uri = format!("/alerts/config/enablement/{name}/{PRODUCT}");

    let (status, first) = call_with_state(
        post(
            &uri,
            json!({ "is_enabled": true, "category": "success_rate" }),
        ),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["is_enabled"], true);

    let (status, second) =
        call_with_state(post(&uri, json!({ "is_enabled": false })), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["is_enabled"], false);

    let (status, read) = call_with_state(get(&uri), &state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["is_enabled"], false);

    let (_, listed) = call_with_state(get("/alerts/config/enablement"), &state).await;
    let matching = listed["enablements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["name"] == name.as_str())
        .count();
    assert_eq!(matching, 1, "the upsert wrote a second row");

    forget(&state, &name).await;
}

#[actix_web::test]
async fn the_definition_switch_wins_over_the_enablement_switch() {
    with_database!(state);
    let name = unique_name("cfg_prec");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, false)),
        &state,
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();
    let uri = format!("/alerts/config/enablement/{name}/{PRODUCT}");

    let (_, enabled) = call_with_state(post(&uri, json!({ "is_enabled": true })), &state).await;
    assert_eq!(enabled["is_enabled"], true);
    assert_eq!(enabled["effective_is_enabled"], false);

    call_with_state(
        post(
            &format!("/alerts/config/definitions/{id}"),
            json!({ "is_enabled": true }),
        ),
        &state,
    )
    .await;
    let (_, read) = call_with_state(get(&uri), &state).await;
    assert_eq!(read["effective_is_enabled"], true);

    let (_, narrowed) = call_with_state(post(&uri, json!({ "is_enabled": false })), &state).await;
    assert_eq!(narrowed["effective_is_enabled"], false);

    forget(&state, &name).await;
}

#[actix_web::test]
async fn an_enablement_row_cannot_name_an_alert_that_does_not_exist() {
    with_database!(state);
    let name = unique_name("cfg_missing");

    let (status, body) = call_with_state(
        post(
            &format!("/alerts/config/enablement/{name}/{PRODUCT}"),
            json!({ "is_enabled": true }),
        ),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_07");
    assert!(!body.to_string().contains(&name));
}

#[actix_web::test]
async fn the_reserved_all_definition_has_no_enablement_of_its_own() {
    with_database!(state);
    let product = unique_name("cfg_allsw");

    call_with_state(
        post(
            "/alerts/config/definitions",
            json!({
                "name": "all",
                "product": product,
                "is_enabled": true,
                "author": "reliability_team"
            }),
        ),
        &state,
    )
    .await;

    let (status, body) = call_with_state(
        post(
            &format!("/alerts/config/enablement/all/{product}"),
            json!({ "is_enabled": true }),
        ),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_07");

    let connection = state.database_connection().await.expect("a connection");
    diesel::sql_query(format!(
        "DELETE FROM alerts_info WHERE product = '{product}'"
    ))
    .execute_async(connection.raw_connection())
    .await
    .expect("the cleanup statement should run");
}

#[actix_web::test]
async fn an_unknown_enablement_key_is_a_404() {
    with_database!(state);

    let (status, body) = call_with_state(
        get("/alerts/config/enablement/no_such_alert_here/payments"),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "IR_06");
}

#[actix_web::test]
async fn an_empty_result_is_a_200_with_a_count() {
    with_database!(state);

    let (status, body) = call_with_state(get("/alerts/config/definitions"), &state).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["count"].is_number());
    assert!(body["definitions"].is_array());
}

async fn stored_rows(state: &AppState, name: &str) -> Vec<(Option<String>, Option<bool>)> {
    let connection = state.database_connection().await.unwrap();

    alerts_dicts::table
        .filter(alerts_dicts::name.eq(name.to_owned()))
        .select((alerts_dicts::key_, alerts_dicts::is_enabled))
        .order(alerts_dicts::ts_created.asc())
        .get_results_async(connection.raw_connection())
        .await
        .unwrap()
}

async fn forget_dictionary(state: &AppState, name: &str) {
    let connection = state.database_connection().await.unwrap();

    diesel::delete(alerts_dicts::table.filter(alerts_dicts::name.eq(name.to_owned())))
        .execute_async(connection.raw_connection())
        .await
        .unwrap();
}

async fn forget_watermark(state: &AppState, user: &str) {
    let connection = state.database_connection().await.unwrap();

    diesel::delete(
        notification_reads::table.filter(notification_reads::user_name.eq(user.to_owned())),
    )
    .execute_async(connection.raw_connection())
    .await
    .unwrap();
}

#[actix_web::test]
#[ignore]
async fn an_entry_saved_twice_updates_the_live_row_rather_than_duplicating_it() {
    let state = state().await;
    let name = unique("mappers");

    for values in ["[\"one\"]", "[\"one\",\"two\"]"] {
        let (status, body) = call_with_state(
            post(
                "/alerts/config/dictionary",
                json!({ "name": name, "key_": "slack_users", "values_": values }),
            ),
            &state,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "saved");
    }

    let rows = stored_rows(&state, &name).await;
    assert_eq!(rows.len(), 1, "a repeated save added a row: {rows:?}");

    let (_, body) = call_with_state(
        get(&format!("/alerts/config/dictionary/{name}/slack_users")),
        &state,
    )
    .await;
    assert_eq!(body["status"], "found");
    assert_eq!(body["entry"]["values_"], "[\"one\",\"two\"]");

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn the_json_columns_come_back_byte_identical() {
    let state = state().await;
    let name = unique("mappers");
    let values = r#"{"b": 1, "a": [2, 3], "c": 1.50}"#;
    let product = r#"["payments"]"#;
    let metadata = r#"{"category": "dashboard"}"#;

    let body = format!(
        r#"{{"name": "{name}", "key_": "spacing", "product": {product}, "values_": {values}, "metadata": {metadata}}}"#
    );
    let (status, saved) = call_raw(
        TestRequest::post()
            .uri("/alerts/config/dictionary")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .insert_header(("content-type", "application/json"))
            .set_payload(body),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, read) = call_raw(
        get(&format!("/alerts/config/dictionary/{name}/spacing")),
        &state,
    )
    .await;

    for response in [&saved, &read] {
        assert!(
            response.contains(&format!(r#""values_":{values}"#)),
            "values_ was re-encoded: {response}"
        );
        assert!(
            response.contains(&format!(r#""product":{product}"#)),
            "product was re-encoded: {response}"
        );
        assert!(
            response.contains(&format!(r#""metadata":{metadata}"#)),
            "metadata was re-encoded: {response}"
        );
    }

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn a_retired_entry_leaves_the_dictionary_and_its_row_stays_behind() {
    let state = state().await;
    let name = unique("mappers");

    call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": name, "key_": "channels", "values_": "[]" }),
        ),
        &state,
    )
    .await;

    let (status, body) = call_with_state(
        delete(&format!("/alerts/config/dictionary/{name}/channels")),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "retired");

    let (_, read) = call_with_state(
        get(&format!("/alerts/config/dictionary/{name}/channels")),
        &state,
    )
    .await;
    assert_eq!(read["status"], "absent");
    assert!(read["entry"].is_null());

    assert_eq!(
        stored_rows(&state, &name).await,
        vec![(Some("channels".to_owned()), Some(false))]
    );

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn saving_a_retired_key_again_writes_a_new_row_rather_than_reviving_the_old_one() {
    let state = state().await;
    let name = unique("mappers");

    call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": name, "key_": "channels", "values_": "[\"old\"]" }),
        ),
        &state,
    )
    .await;
    call_with_state(
        delete(&format!("/alerts/config/dictionary/{name}/channels")),
        &state,
    )
    .await;
    call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": name, "key_": "channels", "values_": "[\"new\"]" }),
        ),
        &state,
    )
    .await;

    let rows = stored_rows(&state, &name).await;
    assert_eq!(
        rows.len(),
        2,
        "the retired row should still be there: {rows:?}"
    );
    assert_eq!(
        rows.iter()
            .filter(|(_, enabled)| *enabled == Some(true))
            .count(),
        1,
        "exactly one row may be live: {rows:?}"
    );

    let (_, body) = call_with_state(
        get(&format!("/alerts/config/dictionary/{name}/channels")),
        &state,
    )
    .await;
    assert_eq!(body["entry"]["values_"], "[\"new\"]");

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn retiring_an_entry_that_is_not_there_is_reported_rather_than_refused() {
    let state = state().await;
    let name = unique("mappers");

    let (status, body) = call_with_state(
        delete(&format!("/alerts/config/dictionary/{name}/missing")),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "absent");
}

#[actix_web::test]
#[ignore]
async fn reading_an_entry_that_is_not_there_is_an_answer_and_not_a_404() {
    let state = state().await;
    let name = unique("mappers");

    let (status, body) = call_with_state(
        get(&format!("/alerts/config/dictionary/{name}/missing")),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "absent");
    assert!(body["entry"].is_null());
}

#[actix_web::test]
#[ignore]
async fn an_entry_whose_key_needs_encoding_is_still_addressable() {
    let state = state().await;
    let name = unique("mappers");

    call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": name, "key_": "sr/drop", "values_": "[]" }),
        ),
        &state,
    )
    .await;

    let (status, body) = call_with_state(
        get(&format!("/alerts/config/dictionary/{name}/sr%2Fdrop")),
        &state,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "found");
    assert_eq!(body["entry"]["key_"], "sr/drop");

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn a_saved_entry_appears_in_the_list() {
    let state = state().await;
    let name = unique("mappers");

    call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": name, "key_": "listed", "values_": "[]" }),
        ),
        &state,
    )
    .await;

    let (status, body) = call_with_state(get("/alerts/config/dictionary"), &state).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "found");
    let entries = body["entries"].as_array().unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry["name"] == name.as_str() && entry["key_"] == "listed"),
        "the saved entry was not listed"
    );

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn a_save_without_a_user_header_is_attributed_to_the_column_default() {
    let state = state().await;
    let name = unique("mappers");

    let (_, body) = call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": name, "key_": "anonymous", "values_": "[]" }),
        ),
        &state,
    )
    .await;

    assert_eq!(body["entry"]["username"], DEFAULT_USERNAME);

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn a_save_records_the_user_the_caller_named() {
    let state = state().await;
    let name = unique("mappers");

    let (_, body) = call_with_state(
        post(
            "/alerts/config/dictionary",
            json!({ "name": name, "key_": "attributed", "values_": "[]" }),
        )
        .insert_header((X_USER_NAME, "ops@example.com")),
        &state,
    )
    .await;

    assert_eq!(body["entry"]["username"], "ops@example.com");

    forget_dictionary(&state, &name).await;
}

#[actix_web::test]
#[ignore]
async fn a_watermark_is_absent_until_the_feed_is_cleared_and_readable_after() {
    let state = state().await;
    let user = unique("user");

    let (status, before) = call_with_state(
        get("/alerts/config/notifications/read").insert_header((X_USER_NAME, user.clone())),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(before["status"], "absent");
    assert!(before["last_read_at"].is_null());

    let (status, marked) = call_with_state(
        TestRequest::post()
            .uri("/alerts/config/notifications/read")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .insert_header((X_USER_NAME, user.clone())),
        &state,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(marked["status"], "found");
    assert!(marked["last_read_at"].is_string());

    let (_, after) = call_with_state(
        get("/alerts/config/notifications/read").insert_header((X_USER_NAME, user.clone())),
        &state,
    )
    .await;
    assert_eq!(after["status"], "found");
    assert_eq!(after["last_read_at"], marked["last_read_at"]);

    forget_watermark(&state, &user).await;
}

#[actix_web::test]
#[ignore]
async fn one_user_clearing_the_feed_leaves_another_users_watermark_alone() {
    let state = state().await;
    let (first, second) = (unique("user"), unique("user"));

    call_with_state(
        TestRequest::post()
            .uri("/alerts/config/notifications/read")
            .insert_header((X_INTERNAL_API_KEY, API_KEY))
            .insert_header((X_USER_NAME, first.clone())),
        &state,
    )
    .await;

    let (_, other) = call_with_state(
        get("/alerts/config/notifications/read").insert_header((X_USER_NAME, second.clone())),
        &state,
    )
    .await;

    assert_eq!(other["status"], "absent");

    forget_watermark(&state, &first).await;
    forget_watermark(&state, &second).await;
}

#[actix_web::test]
#[ignore]
async fn clearing_the_feed_twice_moves_the_watermark_forward() {
    let state = state().await;
    let user = unique("user");

    let mark = || {
        call_with_state(
            TestRequest::post()
                .uri("/alerts/config/notifications/read")
                .insert_header((X_INTERNAL_API_KEY, API_KEY))
                .insert_header((X_USER_NAME, user.clone())),
            &state,
        )
    };

    let (_, first) = mark().await;
    let (status, second) = mark().await;

    assert_eq!(status, StatusCode::OK);
    assert!(second["last_read_at"].as_str() >= first["last_read_at"].as_str());

    forget_watermark(&state, &user).await;
}
