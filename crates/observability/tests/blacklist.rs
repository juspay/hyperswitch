//! Alert blacklist tests through the authenticated Actix route tree.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::{Arc, Mutex};

use actix_web::{http::StatusCode, test, App};
use diesel::{
    Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use diesel_models::{
    errors::DatabaseError,
    observability::{alert_manager::blacklist::BlacklistEntry, schema::alert_blacklist},
    StorageResult,
};
use error_stack::report;
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::{
        alerts_info::AlertsInfoInterface, blacklist::BlacklistInterface,
        thresholds::ThresholdsInterface, StorageInterface,
    },
    domain::notifier::Registry,
    domain_models::{alerts_info, blacklist, thresholds},
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};
use time::{macros::datetime, PrimitiveDateTime};

const API_KEY: &str = "blacklist_test_key";

#[derive(Default)]
struct MemoryStore {
    rows: Mutex<Vec<blacklist::BlacklistEntry>>,
    max_active_rules: Option<i64>,
}

impl StorageInterface for MemoryStore {}

#[async_trait::async_trait]
impl AlertsInfoInterface for MemoryStore {
    async fn insert_alert_info(
        &self,
        _new: alerts_info::AlertsInfoNew,
    ) -> StorageResult<alerts_info::AlertsInfo> {
        Err(report!(DatabaseError::Others))
    }
}

#[async_trait::async_trait]
impl ThresholdsInterface for MemoryStore {
    async fn list_threshold_overrides(&self) -> StorageResult<Vec<thresholds::ThresholdOverride>> {
        Err(report!(DatabaseError::Others))
    }

    async fn upsert_threshold_override(
        &self,
        _new: thresholds::ThresholdOverrideNew,
        _max_active_rules: i64,
    ) -> StorageResult<thresholds::ThresholdUpsertOutcome> {
        Err(report!(DatabaseError::Others))
    }

    async fn delete_threshold_override(
        &self,
        _new: thresholds::ThresholdOverrideNew,
    ) -> StorageResult<thresholds::ThresholdOverride> {
        Err(report!(DatabaseError::Others))
    }
}

fn now() -> PrimitiveDateTime {
    datetime!(2026-09-15 09:45:00)
}

#[async_trait::async_trait]
impl BlacklistInterface for MemoryStore {
    async fn list_blacklist_entries(&self) -> StorageResult<Vec<blacklist::BlacklistEntry>> {
        let mut rows = self.rows.lock().unwrap().clone();
        rows.retain(|row| !row.is_deleted);
        rows.sort_by(|a, b| (&a.merchant_id, &a.profile_id).cmp(&(&b.merchant_id, &b.profile_id)));
        Ok(rows)
    }

    async fn upsert_blacklist_entry(
        &self,
        new: blacklist::BlacklistEntryNew,
        max_active_rules: i64,
    ) -> StorageResult<blacklist::BlacklistUpsertOutcome> {
        let max_active_rules = self.max_active_rules.unwrap_or(max_active_rules);
        let mut rows = self.rows.lock().unwrap();
        let existing = rows.iter().position(|row| {
            (&row.rule_id, &row.merchant_id, &row.profile_id)
                == (&new.rule_id, &new.merchant_id, &new.profile_id)
        });
        let already_active = existing.is_some_and(|index| !rows[index].is_deleted);
        let active_count = rows.iter().filter(|row| !row.is_deleted).count() as i64;
        if !already_active && active_count >= max_active_rules {
            return Ok(blacklist::BlacklistUpsertOutcome::ActiveRuleLimitReached);
        }
        let stored = blacklist::BlacklistEntry {
            rule_id: new.rule_id,
            merchant_id: new.merchant_id,
            profile_id: new.profile_id,
            reason: new.reason,
            created_by: new.created_by,
            last_updated_at: now(),
            is_deleted: false,
        };
        if let Some(index) = existing {
            rows[index] = stored.clone();
        } else {
            rows.push(stored.clone());
        }
        Ok(blacklist::BlacklistUpsertOutcome::Stored(stored))
    }

    async fn delete_blacklist_entry(
        &self,
        new: blacklist::BlacklistEntryNew,
    ) -> StorageResult<blacklist::BlacklistEntry> {
        let mut rows = self.rows.lock().unwrap();
        let tombstone = blacklist::BlacklistEntry {
            rule_id: new.rule_id,
            merchant_id: new.merchant_id,
            profile_id: new.profile_id,
            reason: new.reason,
            created_by: new.created_by,
            last_updated_at: now(),
            is_deleted: true,
        };
        if let Some(row) = rows.iter_mut().find(|row| {
            (&row.rule_id, &row.merchant_id, &row.profile_id)
                == (
                    &tombstone.rule_id,
                    &tombstone.merchant_id,
                    &tombstone.profile_id,
                )
        }) {
            *row = tombstone.clone();
        } else {
            rows.push(tombstone.clone());
        }
        Ok(tombstone)
    }
}

fn state(max_active_rules: i64) -> AppState {
    let conf: observability::Settings = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY }
    }))
    .unwrap();
    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::default()),
        email: Arc::new(Registry::default()),
        metrics: None,
        store: Arc::new(MemoryStore {
            rows: Mutex::new(Vec::new()),
            max_active_rules: Some(max_active_rules),
        }),
    }
}

async fn call_raw(
    state: AppState,
    method: actix_web::http::Method,
    key: Option<&str>,
    payload: Option<&[u8]>,
) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state))).await;
    let mut request = test::TestRequest::default()
        .method(method)
        .uri("/alerts/blacklist");
    if let Some(key) = key {
        request = request.insert_header((X_INTERNAL_API_KEY, key));
    }
    if let Some(payload) = payload {
        request = request
            .insert_header(("content-type", "application/json"))
            .set_payload(payload.to_owned());
    }
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let body = test::read_body(response).await;
    (status, serde_json::from_slice(&body).unwrap())
}

async fn call(
    state: AppState,
    method: actix_web::http::Method,
    key: Option<&str>,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let encoded = payload.map(|value| serde_json::to_vec(&value).unwrap());
    call_raw(state, method, key, encoded.as_deref()).await
}

fn body(merchant_id: &str) -> Value {
    json!({
        "rule_id": "all", "merchant_id": merchant_id, "profile_id": "",
        "reason": "dashboard is_blacklisted", "created_by": "dashboard"
    })
}

#[actix_web::test]
async fn crud_tombstone_reactivation_and_ordering_preserve_the_contract() {
    let state = state(5000);
    for merchant in ["merchant_b", " merchant_a "] {
        let (status, response) = call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body(merchant)),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response["status"], "SUCCESS");
    }

    let (_, listed) = call(
        state.clone(),
        actix_web::http::Method::GET,
        Some(API_KEY),
        None,
    )
    .await;
    let entries = listed["entries"].as_array().unwrap();
    assert_eq!(entries[0]["merchant_id"], "merchant_a");
    assert_eq!(entries[1]["merchant_id"], "merchant_b");
    assert_eq!(entries[0]["last_updated_at"], "2026-09-15T09:45:00.000Z");
    assert!(entries[0].get("is_deleted").is_none());

    let delete = json!({"merchant_id": "merchant_a", "created_by": "dashboard"});
    for _ in 0..2 {
        let (status, response) = call(
            state.clone(),
            actix_web::http::Method::DELETE,
            Some(API_KEY),
            Some(delete.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response, json!({"status": "SUCCESS"}));
    }
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body("merchant_a")),
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[actix_web::test]
async fn active_update_is_allowed_at_cap_but_new_and_reactivated_keys_are_not() {
    let state = state(1);
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body("one"))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body("one"))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body("two"))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );

    let delete = json!({"merchant_id": "missing", "created_by": "dashboard"});
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::DELETE,
            Some(API_KEY),
            Some(delete)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            state,
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body("missing"))
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[actix_web::test]
async fn auth_json_shape_and_scope_validation_are_enforced() {
    for method in [
        actix_web::http::Method::GET,
        actix_web::http::Method::POST,
        actix_web::http::Method::DELETE,
    ] {
        let payload = if method == actix_web::http::Method::POST {
            Some(body("merchant"))
        } else if method == actix_web::http::Method::DELETE {
            Some(json!({"merchant_id": "merchant", "created_by": "dashboard"}))
        } else {
            None
        };
        assert_eq!(
            call(state(10), method, None, payload).await.0,
            StatusCode::UNAUTHORIZED
        );
    }

    assert_eq!(
        call_raw(
            state(10),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(b"{bad")
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut unknown = body("merchant");
    unknown["unknown"] = json!(true);
    assert_eq!(
        call(
            state(10),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(unknown)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            state(10),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body("  "))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            state(10),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(json!({"merchant_id": "merchant", "created_by": "dashboard"})),
        )
        .await
        .0,
        StatusCode::OK
    );
}

fn postgres_database(url: &str) -> Database {
    let parsed = url::Url::parse(url).expect("OBSERVABILITY_TEST_DATABASE_URL must be a URL");
    Database {
        username: parsed.username().to_owned(),
        password: parsed.password().unwrap_or_default().to_owned().into(),
        host: parsed
            .host_str()
            .expect("database URL must include a host")
            .to_owned(),
        port: parsed.port().unwrap_or(5432),
        dbname: parsed.path().trim_start_matches('/').to_owned(),
        pool_size: 8,
        min_idle_pool_size: 1,
        connection_timeout: 10,
    }
}

fn repository_entry(merchant_id: &str, reason: &str) -> blacklist::BlacklistEntryNew {
    blacklist::BlacklistEntryNew {
        rule_id: "all".to_owned(),
        merchant_id: merchant_id.to_owned(),
        profile_id: String::new(),
        reason: reason.to_owned(),
        created_by: "database-test".to_owned(),
        is_deleted: false,
    }
}

/// Exercises the migration-backed Diesel repository, including the serialized cap boundary. Run
/// after applying the observability migrations:
///
/// `OBSERVABILITY_TEST_DATABASE_URL=postgres://... cargo test -p observability --test blacklist postgres_repository -- --ignored`
#[actix_web::test]
#[ignore = "requires OBSERVABILITY_TEST_DATABASE_URL and applied observability migrations"]
async fn postgres_repository_covers_crud_ordering_and_concurrent_cap() {
    let database_url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .expect("OBSERVABILITY_TEST_DATABASE_URL is required");
    let database = postgres_database(&database_url);
    let mut assertion_connection = PgConnection::establish(&database_url).unwrap();
    diesel::delete(alert_blacklist::table)
        .execute(&mut assertion_connection)
        .unwrap();

    let store = Arc::new(observability::db::Store::new(&database).await.unwrap());

    // Inserts are returned in merchant/profile order, and an active key remains writable at cap.
    for merchant in ["merchant_b", "merchant_a"] {
        assert!(matches!(
            store
                .upsert_blacklist_entry(repository_entry(merchant, "initial"), 2)
                .await
                .unwrap(),
            blacklist::BlacklistUpsertOutcome::Stored(_)
        ));
    }
    let listed = store.list_blacklist_entries().await.unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|row| row.merchant_id.as_str())
            .collect::<Vec<_>>(),
        ["merchant_a", "merchant_b"]
    );
    assert!(matches!(
        store
            .upsert_blacklist_entry(repository_entry("merchant_a", "replacement"), 2)
            .await
            .unwrap(),
        blacklist::BlacklistUpsertOutcome::Stored(row) if row.reason == "replacement"
    ));

    // Tombstones are persisted and can be reactivated when capacity is available.
    let mut tombstone = repository_entry("merchant_b", "");
    tombstone.created_by = "database-test-deleter".to_owned();
    tombstone.is_deleted = true;
    store.delete_blacklist_entry(tombstone).await.unwrap();
    let deleted = alert_blacklist::table
        .filter(alert_blacklist::merchant_id.eq("merchant_b"))
        .select(BlacklistEntry::as_select())
        .first::<BlacklistEntry>(&mut assertion_connection)
        .unwrap();
    assert!(deleted.is_deleted);
    assert_eq!(deleted.reason, "");
    assert_eq!(deleted.created_by, "database-test-deleter");
    assert!(matches!(
        store
            .upsert_blacklist_entry(repository_entry("merchant_b", "reactivated"), 2)
            .await
            .unwrap(),
        blacklist::BlacklistUpsertOutcome::Stored(row) if row.reason == "reactivated"
    ));

    // Leave one slot, then race two distinct creates for it. Exactly one may become active.
    diesel::delete(alert_blacklist::table)
        .execute(&mut assertion_connection)
        .unwrap();
    store
        .upsert_blacklist_entry(repository_entry("seed", "seed"), 2)
        .await
        .unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let contender = |merchant: &'static str| {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        async move {
            barrier.wait().await;
            store
                .upsert_blacklist_entry(repository_entry(merchant, "racing"), 2)
                .await
                .unwrap()
        }
    };
    let first = tokio::spawn(contender("racer_a"));
    let second = tokio::spawn(contender("racer_b"));
    barrier.wait().await;
    let outcomes = [first.await.unwrap(), second.await.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, blacklist::BlacklistUpsertOutcome::Stored(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(
                outcome,
                blacklist::BlacklistUpsertOutcome::ActiveRuleLimitReached
            ))
            .count(),
        1
    );
    assert_eq!(store.list_blacklist_entries().await.unwrap().len(), 2);
}
