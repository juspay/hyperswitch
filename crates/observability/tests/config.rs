//! The alert configuration routes, exercised through actix as a caller would reach them.
//!
//! These go through the real route tree rather than calling handlers directly, for the same reason
//! `tests/notify.rs` does: most of what these tickets decided lives *between* the handler and the
//! caller — the guard, the path extractors, the body extractor's rejection shape, and which
//! failures are which status code.
//!
//! ## Two kinds of test here
//!
//! The tests below the divider need a database. They are skipped, loudly, when one is not
//! reachable — CI has no Postgres for this crate, and a suite that fails there would be turned off
//! rather than fixed. Point them at a database with:
//!
//! ```text
//! OBSERVABILITY_TEST_DATABASE_URL=postgres://db_user:db_pass@127.0.0.1:5432/observability \
//!     cargo test -p observability --test config
//! ```
//!
//! The tests above it need none, and that is not a compromise: the guard, the malformed body and
//! the unreachable-database answer are all properties of the route tree, and the last of the three
//! is *only* observable with a pool that cannot connect.
//!
//! Rows are named with a per-test unique suffix, so a rerun does not collide with the last run and
//! two tests running in parallel do not see each other's definitions.

// `print_stderr`: a skipped test has to say so somewhere a developer will see it, and a test
// harness has no logger.
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
};

use actix_web::{
    http::StatusCode,
    test::{self, TestRequest},
    App,
};
use async_bb8_diesel::AsyncRunQueryDsl;
use observability::{
    auth::X_INTERNAL_API_KEY,
    domain::notifier::Registry,
    routes::Alerts,
    settings::DatabaseSettings,
    state::{build_database_pool, AppState},
};
use serde_json::{json, Value};

const API_KEY: &str = "test_internal_key";
const PRODUCT: &str = "payments";

/// A pool pointing at an address nothing answers on, for the routes that must say the store is
/// away rather than say it is empty.
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

async fn call_with_state(request: TestRequest, state: AppState) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state))).await;
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

// ---------------------------------------------------------------------------
// Properties of the route tree, which need no database
// ---------------------------------------------------------------------------

/// Every body here parses. That is deliberate: actix runs the body extractor before the handler
/// and the guard runs inside it, so an unparseable body is answered `400` without the key being
/// checked — a known ordering `tests/notify.rs` records. Sending a valid body is what makes the
/// guard, rather than the extractor, the thing being tested.
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
        let (status, body) = call_with_state(request.uri(&uri), state_with(&unreachable())).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri} must be guarded");
        assert_eq!(body["error"]["code"], "IR_01");
    }
}

/// The property both tickets turn on. "Nothing is configured" and "the configuration store is
/// away" are the same sentence to a caller that only reads the status, and the alert manager's
/// outage rule reads an empty list as all-clear.
#[actix_web::test]
async fn an_unreachable_database_is_a_503_and_never_an_empty_list() {
    for uri in ["/alerts/config/definitions", "/alerts/config/enablement"] {
        let (status, body) = call_with_state(get(uri), state_with(&unreachable())).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{uri}");
        assert_eq!(body["error"]["code"], "HE_01");
        assert!(body.get("count").is_none());
    }
}

/// The host, the database and the role are all in a connection error, and this port is reachable
/// by anything that can reach the service.
#[actix_web::test]
async fn an_unreachable_database_does_not_describe_itself_to_the_caller() {
    let (_, body) = call_with_state(
        get("/alerts/config/definitions"),
        state_with(&unreachable()),
    )
    .await;
    let rendered = body.to_string();

    assert!(!rendered.contains("unused"));
    assert!(!rendered.contains("127.0.0.1"));
}

/// A malformed body renders in the crate's envelope rather than actix's own plain-text 400, and it
/// is rejected before a connection is ever leased.
#[actix_web::test]
async fn a_definition_missing_a_required_field_is_a_400_in_our_shape() {
    let (status, body) = call_with_state(
        post(
            "/alerts/config/definitions",
            json!({ "name": "sr_drop", "product": PRODUCT }),
        ),
        state_with(&unreachable()),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_04");
    assert_eq!(body["error"]["type"], "invalid_request");
}

/// A path that is not a uuid must not reach the query layer as a string that happens to parse
/// later, or "no such definition" and "that is not an id" would be the same answer.
#[actix_web::test]
async fn a_definition_id_that_is_not_a_uuid_does_not_route() {
    let (status, _) = call_with_state(
        get("/alerts/config/definitions/sr_drop"),
        state_with(&unreachable()),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// End to end against a real database
// ---------------------------------------------------------------------------

/// The database these tests use, or `None` when there is not one.
///
/// Defaults match `config/observability.toml`, so a developer who has run `just
/// migrate_observability` needs no environment variable.
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

    // The lease is dropped inside `map` so that nothing borrowed from the pool outlives this
    // statement — `state` is moved out of the function on the next line.
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

/// A name no other test or earlier run will have used.
fn unique_name(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    format!(
        "{prefix}_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// Remove what a test wrote. There is no delete route by design, so this reaches past the API.
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

/// A definition with every field of every entry spelled out, so that a round trip through the
/// database is an equality rather than an approximation.
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
        state.clone(),
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

    let (status, read) = call_with_state(
        get(&format!("/alerts/config/definitions/{id}")),
        state.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read, created);

    let (status, listed) = call_with_state(get("/alerts/config/definitions"), state.clone()).await;
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

/// The structured columns are the point of the resource: what the alert manager reads back has to
/// be what the dashboard meant, entry for entry and value for value.
#[actix_web::test]
async fn the_json_columns_survive_a_round_trip_through_the_database() {
    with_database!(state);
    let name = unique_name("cfg_json");

    let sent = definition_body(&name, true);
    let (_, created) = call_with_state(
        post("/alerts/config/definitions", sent.clone()),
        state.clone(),
    )
    .await;

    for column in ["blacklist", "snooze", "thresholds", "metadata"] {
        assert_eq!(created[column], sent[column], "{column} did not round trip");
    }

    forget(&state, &name).await;
}

/// The cost of typing the columns, recorded rather than discovered. A partial entry comes back
/// filled in — `profile_id` as the empty string that means "every profile", `created_by` as an
/// explicit null — because the value has been through `serde_json` in both directions. That is the
/// trade the ticket flagged: the bytes are not preserved, and in exchange a shape the alert manager
/// cannot read is refused at the edge rather than stored and silently ignored.
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
        state.clone(),
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

/// The whole reason an update is partial: the suppression screen and the threshold screen edit the
/// same row, and neither may discard what the other just saved.
#[actix_web::test]
async fn an_update_leaves_the_columns_it_did_not_mention_alone() {
    with_database!(state);
    let name = unique_name("cfg_partial");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        state.clone(),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();

    let (status, updated) = call_with_state(
        post(
            &format!("/alerts/config/definitions/{id}"),
            json!({ "thresholds": [{ "merchant_id": "merchant_9999", "tolerance": 5.0 }] }),
        ),
        state.clone(),
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

/// An explicit null is a different request from an absent field, and only a write proves the
/// distinction survives all the way into the column.
#[actix_web::test]
async fn an_explicit_null_clears_a_column_that_an_absent_field_would_have_kept() {
    with_database!(state);
    let name = unique_name("cfg_null");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        state.clone(),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();

    let (_, kept) = call_with_state(
        post(&format!("/alerts/config/definitions/{id}"), json!({})),
        state.clone(),
    )
    .await;
    assert_eq!(kept["period"], 15);

    let (_, cleared) = call_with_state(
        post(
            &format!("/alerts/config/definitions/{id}"),
            json!({ "period": null, "blacklist": null }),
        ),
        state.clone(),
    )
    .await;
    assert_eq!(cleared["period"], Value::Null);
    assert_eq!(cleared["blacklist"], json!([]));

    forget(&state, &name).await;
}

/// `is_enabled` is how an alert is turned off, which is only true if it can be turned back on.
#[actix_web::test]
async fn an_alert_is_turned_off_and_on_again_through_is_enabled() {
    with_database!(state);
    let name = unique_name("cfg_switch");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        state.clone(),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();

    for expected in [false, true] {
        let (status, updated) = call_with_state(
            post(
                &format!("/alerts/config/definitions/{id}"),
                json!({ "is_enabled": expected }),
            ),
            state.clone(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["is_enabled"], expected);
    }

    forget(&state, &name).await;
}

/// A name resolving to two rows would make the alert manager's lookup and the enablement table's
/// reference both "pick one".
#[actix_web::test]
async fn a_second_definition_with_the_same_name_and_product_is_refused() {
    with_database!(state);
    let name = unique_name("cfg_dup");

    let (first, _) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        state.clone(),
    )
    .await;
    let (second, body) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, false)),
        state.clone(),
    )
    .await;

    assert_eq!(first, StatusCode::OK);
    assert_eq!(second, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_05");

    forget(&state, &name).await;
}

/// The reserved row is a definition like any other: it is created, listed and edited through the
/// same routes, because the suppression it carries has to be manageable.
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
        state.clone(),
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
        state,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "IR_03");
}

/// The edge case the composite key exists for: the second call must be an update, not a second row
/// disagreeing with the first about whether the alert is on.
#[actix_web::test]
async fn a_repeated_enablement_upsert_updates_rather_than_duplicating() {
    with_database!(state);
    let name = unique_name("cfg_enable");

    let (_, _) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, true)),
        state.clone(),
    )
    .await;

    let uri = format!("/alerts/config/enablement/{name}/{PRODUCT}");

    let (status, first) = call_with_state(
        post(
            &uri,
            json!({ "is_enabled": true, "category": "success_rate" }),
        ),
        state.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["is_enabled"], true);

    let (status, second) =
        call_with_state(post(&uri, json!({ "is_enabled": false })), state.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["is_enabled"], false);

    let (status, read) = call_with_state(get(&uri), state.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["is_enabled"], false);

    let (_, listed) = call_with_state(get("/alerts/config/enablement"), state.clone()).await;
    let matching = listed["enablements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["name"] == name.as_str())
        .count();
    assert_eq!(matching, 1, "the upsert wrote a second row");

    forget(&state, &name).await;
}

/// The decision this ticket asked to be written down, proved rather than described: the definition
/// is the master switch, and the enablement row can only narrow it.
#[actix_web::test]
async fn the_definition_switch_wins_over_the_enablement_switch() {
    with_database!(state);
    let name = unique_name("cfg_prec");

    let (_, created) = call_with_state(
        post("/alerts/config/definitions", definition_body(&name, false)),
        state.clone(),
    )
    .await;
    let id = created["id"].as_str().unwrap().to_owned();
    let uri = format!("/alerts/config/enablement/{name}/{PRODUCT}");

    // The narrower switch is on and the definition is off: the alert does not run.
    let (_, enabled) =
        call_with_state(post(&uri, json!({ "is_enabled": true })), state.clone()).await;
    assert_eq!(enabled["is_enabled"], true);
    assert_eq!(enabled["effective_is_enabled"], false);

    // Turning the definition on is what makes it run.
    call_with_state(
        post(
            &format!("/alerts/config/definitions/{id}"),
            json!({ "is_enabled": true }),
        ),
        state.clone(),
    )
    .await;
    let (_, read) = call_with_state(get(&uri), state.clone()).await;
    assert_eq!(read["effective_is_enabled"], true);

    // And the narrower switch can still turn it off again.
    let (_, narrowed) =
        call_with_state(post(&uri, json!({ "is_enabled": false })), state.clone()).await;
    assert_eq!(narrowed["effective_is_enabled"], false);

    forget(&state, &name).await;
}

/// r-apps enforces this with a database trigger this schema does not have. Without the check, a
/// switch can be wired to an alert nobody defined and looks on the screen exactly like one that
/// works.
#[actix_web::test]
async fn an_enablement_row_cannot_name_an_alert_that_does_not_exist() {
    with_database!(state);
    let name = unique_name("cfg_missing");

    let (status, body) = call_with_state(
        post(
            &format!("/alerts/config/enablement/{name}/{PRODUCT}"),
            json!({ "is_enabled": true }),
        ),
        state,
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "IR_07");
    assert!(!body.to_string().contains(&name));
}

/// The reserved row carries suppression for every detector and is not a detector, so there is
/// nothing for a switch on it to turn on or off.
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
        state.clone(),
    )
    .await;

    let (status, body) = call_with_state(
        post(
            &format!("/alerts/config/enablement/all/{product}"),
            json!({ "is_enabled": true }),
        ),
        state.clone(),
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
        state,
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "IR_06");
}

/// The other half of the outage rule: a database that answers with nothing must be a `200` with a
/// count of zero, or every empty list would look like an incident.
#[actix_web::test]
async fn an_empty_result_is_a_200_with_a_count() {
    with_database!(state);

    let (status, body) = call_with_state(get("/alerts/config/definitions"), state).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["count"].is_number());
    assert!(body["definitions"].is_array());
}
