//! Alert lifecycle state tests through the authenticated Actix route tree.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use actix_web::{http::StatusCode, test, App};
use diesel::{
    Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use diesel_models::{
    errors::DatabaseError,
    observability::{
        alert_manager::lifecycle_events::LifecycleEvent as StorageLifecycleEvent,
        schema::alert_lifecycle_events,
    },
    StorageResult,
};
use error_stack::report;
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::{
        alerts_info::AlertsInfoInterface, blacklist::BlacklistInterface,
        dictionary::DictionaryInterface, lifecycle_events::LifecycleEventsInterface,
        metadata::AlertMetadataInterface, rule_toggles::RuleTogglesInterface,
        thresholds::ThresholdsInterface, StorageInterface,
    },
    domain::notifier::Registry,
    domain_models::{
        alerts_info, blacklist, dictionary, lifecycle_events, metadata, rule_toggles, thresholds,
    },
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};
use time::{macros::datetime, Duration, PrimitiveDateTime};

const API_KEY: &str = "lifecycle_test_key";

#[derive(Default)]
struct MemoryStore {
    rows: Mutex<Vec<lifecycle_events::LifecycleEvent>>,
}

struct FailingStore;

impl StorageInterface for MemoryStore {}
impl StorageInterface for FailingStore {}

macro_rules! impl_unused_interfaces {
    ($store:ty) => {
        #[async_trait::async_trait]
        impl AlertsInfoInterface for $store {
            async fn insert_alert_info(
                &self,
                _: alerts_info::AlertsInfoNew,
            ) -> StorageResult<alerts_info::AlertsInfo> {
                Err(report!(DatabaseError::Others))
            }
        }
        #[async_trait::async_trait]
        impl BlacklistInterface for $store {
            async fn list_blacklist_entries(
                &self,
            ) -> StorageResult<Vec<blacklist::BlacklistEntry>> {
                Err(report!(DatabaseError::Others))
            }
            async fn upsert_blacklist_entry(
                &self,
                _: blacklist::BlacklistEntryNew,
                _: i64,
            ) -> StorageResult<blacklist::BlacklistUpsertOutcome> {
                Err(report!(DatabaseError::Others))
            }
            async fn delete_blacklist_entry(
                &self,
                _: blacklist::BlacklistEntryNew,
            ) -> StorageResult<blacklist::BlacklistEntry> {
                Err(report!(DatabaseError::Others))
            }
        }
        #[async_trait::async_trait]
        impl DictionaryInterface for $store {
            async fn list_dictionary_entries(
                &self,
            ) -> StorageResult<Vec<dictionary::DictionaryEntry>> {
                Err(report!(DatabaseError::Others))
            }
            async fn upsert_dictionary_entry(
                &self,
                _: dictionary::DictionaryEntryNew,
            ) -> StorageResult<dictionary::DictionaryEntry> {
                Err(report!(DatabaseError::Others))
            }
        }
        #[async_trait::async_trait]
        impl AlertMetadataInterface for $store {
            async fn list_alert_metadata(
                &self,
            ) -> StorageResult<Vec<metadata::AlertMetadataEntry>> {
                Err(report!(DatabaseError::Others))
            }
            async fn patch_alert_metadata(
                &self,
                _: metadata::AlertMetadataPatch,
            ) -> StorageResult<metadata::AlertMetadataEntry> {
                Err(report!(DatabaseError::Others))
            }
        }
        #[async_trait::async_trait]
        impl RuleTogglesInterface for $store {
            async fn list_rule_toggles(&self) -> StorageResult<Vec<rule_toggles::RuleToggle>> {
                Err(report!(DatabaseError::Others))
            }
            async fn set_rule_toggle(
                &self,
                _: rule_toggles::RuleToggleNew,
            ) -> StorageResult<rule_toggles::RuleToggle> {
                Err(report!(DatabaseError::Others))
            }
        }
        #[async_trait::async_trait]
        impl ThresholdsInterface for $store {
            async fn list_threshold_overrides(
                &self,
            ) -> StorageResult<Vec<thresholds::ThresholdOverride>> {
                Err(report!(DatabaseError::Others))
            }
            async fn upsert_threshold_override(
                &self,
                _: thresholds::ThresholdOverrideNew,
                _: i64,
            ) -> StorageResult<thresholds::ThresholdUpsertOutcome> {
                Err(report!(DatabaseError::Others))
            }
            async fn delete_threshold_override(
                &self,
                _: thresholds::ThresholdOverrideNew,
            ) -> StorageResult<thresholds::ThresholdOverride> {
                Err(report!(DatabaseError::Others))
            }
        }
    };
}
impl_unused_interfaces!(MemoryStore);
impl_unused_interfaces!(FailingStore);

#[async_trait::async_trait]
impl LifecycleEventsInterface for MemoryStore {
    async fn list_lifecycle_events(
        &self,
        from: Option<PrimitiveDateTime>,
        to: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<lifecycle_events::LifecycleEvent>> {
        let mut rows: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| {
                from.is_none_or(|from| row.last_seen >= from)
                    && to.is_none_or(|to| row.first_seen <= to)
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| a.alert_key.cmp(&b.alert_key));
        Ok(rows)
    }

    async fn replace_lifecycle_events(
        &self,
        batch: lifecycle_events::LifecycleEventsBatch,
    ) -> StorageResult<usize> {
        let persisted = batch.events.len();
        let mut rows = self.rows.lock().unwrap();
        for event in batch.events {
            if let Some(existing) = rows.iter_mut().find(|row| row.alert_key == event.alert_key) {
                if event.last_updated_at > existing.last_updated_at {
                    *existing = event;
                }
            } else {
                rows.push(event);
            }
        }
        rows.retain(|row| row.last_updated_at >= batch.retention_cutoff);
        Ok(persisted)
    }
}

#[async_trait::async_trait]
impl LifecycleEventsInterface for FailingStore {
    async fn list_lifecycle_events(
        &self,
        _: Option<PrimitiveDateTime>,
        _: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<lifecycle_events::LifecycleEvent>> {
        Err(report!(DatabaseError::Others))
    }
    async fn replace_lifecycle_events(
        &self,
        _: lifecycle_events::LifecycleEventsBatch,
    ) -> StorageResult<usize> {
        Err(report!(DatabaseError::Others))
    }
}

fn state_with_store(store: Arc<dyn StorageInterface>) -> AppState {
    let conf: observability::Settings =
        serde_json::from_value(json!({"auth": {"internal_api_key": API_KEY}})).unwrap();
    AppState {
        conf: Arc::new(conf),
        chat: Arc::new(Registry::default()),
        email: Arc::new(Registry::default()),
        metrics: None,
        store,
    }
}

fn state() -> AppState {
    state_with_store(Arc::new(MemoryStore::default()))
}

fn event(alert_key: &str, first_seen: &str, last_seen: &str) -> Value {
    json!({
        "alert_key": alert_key, "detector": "webhook_rejected", "merchant_id": "m1",
        "profile_id": "", "state": "firing", "first_seen": first_seen, "last_seen": last_seen,
        "recovered_at": "1970-01-01T00:00:00Z", "runs": 4, "severity": "critical", "sr": 0.0,
        "failed": 11, "total": 10, "connector": "", "notified_at": last_seen,
        "ts_slack": "", "sent": false
    })
}

async fn call(
    state: AppState,
    method: actix_web::http::Method,
    uri: &str,
    key: Option<&str>,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state))).await;
    let mut request = test::TestRequest::default().method(method).uri(uri);
    if let Some(key) = key {
        request = request.insert_header((X_INTERNAL_API_KEY, key));
    }
    if let Some(payload) = payload {
        request = request.set_json(payload);
    }
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let bytes = test::read_body(response).await;
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
        },
    )
}

#[actix_web::test]
async fn full_batch_round_trip_overlap_and_delivery_replacement() {
    let state = state();
    let key1 = "0123456789abcdef0123456789abcdef";
    let key2 = "fedcba9876543210fedcba9876543210";
    let body = json!({"events": [
        event(key1, "2026-09-15T09:00:00Z", "2026-09-15T09:45:00Z"),
        event(key2, "2026-09-15T10:00:00Z", "2026-09-15T10:15:00Z")
    ], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 0});
    let (status, response) = call(
        state.clone(),
        actix_web::http::Method::POST,
        "/alerts/lifecycle_events/batch",
        Some(API_KEY),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!({"ok": true, "persisted": 2}));

    let (_, listed) = call(
        state.clone(),
        actix_web::http::Method::GET,
        "/alerts/lifecycle_events?from=2026-09-15T09%3A30%3A00Z&to=2026-09-15T09%3A50%3A00Z",
        Some(API_KEY),
        None,
    )
    .await;
    assert_eq!(listed["events"].as_array().unwrap().len(), 1);
    assert_eq!(listed["events"][0]["alert_key"], key1);
    assert!(listed["server_time"].is_string());

    let mut delivered = event(key1, "2026-09-15T09:00:00Z", "2026-09-15T09:45:00Z");
    delivered["sent"] = json!(true);
    delivered["ts_slack"] = json!("1712345.678");
    call(
        state.clone(),
        actix_web::http::Method::POST,
        "/alerts/lifecycle_events/batch",
        Some(API_KEY),
        Some(json!({
            "events": [delivered],
            "snapshot_at": "2026-09-15T10:30:00Z",
            "version_bump_seconds": 1
        })),
    )
    .await;

    // A stale evaluation batch arriving after delivery must not revert delivery state.
    call(
        state.clone(),
        actix_web::http::Method::POST,
        "/alerts/lifecycle_events/batch",
        Some(API_KEY),
        Some(json!({
            "events": [event(key1, "2026-09-15T09:00:00Z", "2026-09-15T09:45:00Z")],
            "snapshot_at": "2026-09-15T10:30:00Z",
            "version_bump_seconds": 0
        })),
    )
    .await;

    let (_, listed) = call(
        state,
        actix_web::http::Method::GET,
        "/alerts/lifecycle_events",
        Some(API_KEY),
        None,
    )
    .await;
    let row = listed["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["alert_key"] == key1)
        .unwrap();
    assert_eq!(row["sent"], true);
    assert_eq!(row["ts_slack"], "1712345.678");
    assert_eq!(row["runs"], 4);
    assert_eq!(row["failed"], 11);
    assert_eq!(row["total"], 10);
}

#[actix_web::test]
async fn authentication_and_request_validation_are_enforced() {
    assert_eq!(
        call(
            state(),
            actix_web::http::Method::GET,
            "/alerts/lifecycle_events",
            None,
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            state(),
            actix_web::http::Method::POST,
            "/alerts/lifecycle_events/batch",
            Some("wrong"),
            Some(json!({
                "events": [], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 0
            }))
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );

    let mut invalid_counters = event(
        "0123456789abcdef0123456789abcdef",
        "2026-09-15T09:00:00Z",
        "2026-09-15T09:45:00Z",
    );
    invalid_counters["failed"] = json!(-1);

    for body in [
        json!({"events": [event("short", "2026-09-15T09:00:00Z", "2026-09-15T09:45:00Z")], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 0}),
        json!({"events": [event("0123456789abcdef0123456789abcdef", "bad", "2026-09-15T09:45:00Z")], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 0}),
        json!({"events": [], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 2}),
        json!({"events": [invalid_counters], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 0}),
        json!({"events": [], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 0, "unknown": true}),
    ] {
        assert_eq!(
            call(
                state(),
                actix_web::http::Method::POST,
                "/alerts/lifecycle_events/batch",
                Some(API_KEY),
                Some(body)
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        call(
            state(),
            actix_web::http::Method::GET,
            "/alerts/lifecycle_events?unknown=x",
            Some(API_KEY),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
}

#[actix_web::test]
async fn storage_failures_return_500() {
    let state = state_with_store(Arc::new(FailingStore));
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::GET,
            "/alerts/lifecycle_events",
            Some(API_KEY),
            None
        )
        .await
        .0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        call(
            state,
            actix_web::http::Method::POST,
            "/alerts/lifecycle_events/batch",
            Some(API_KEY),
            Some(json!({
                "events": [], "snapshot_at": "2026-09-15T10:30:00Z", "version_bump_seconds": 0
            }))
        )
        .await
        .0,
        StatusCode::INTERNAL_SERVER_ERROR
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
        pool_size: 4,
        min_idle_pool_size: 1,
        connection_timeout: 10,
    }
}

fn repository_event(
    alert_key: &str,
    last_updated_at: PrimitiveDateTime,
) -> lifecycle_events::LifecycleEvent {
    lifecycle_events::LifecycleEvent {
        alert_key: alert_key.to_owned(),
        detector: "webhook_rejected".to_owned(),
        merchant_id: "merchant".to_owned(),
        profile_id: String::new(),
        state: "firing".to_owned(),
        first_seen: datetime!(2026-06-01 0:00),
        last_seen: datetime!(2026-06-01 0:05),
        recovered_at: datetime!(1970-01-01 0:00),
        runs: 4,
        severity: "critical".to_owned(),
        sr: 0.0,
        failed: 10,
        total: 10,
        connector: String::new(),
        notified_at: datetime!(2026-06-01 0:05),
        ts_slack: String::new(),
        sent: false,
        last_updated_at,
    }
}

/// Exercises the migration-backed Diesel transaction. Run after applying the observability
/// migrations:
///
/// `OBSERVABILITY_TEST_DATABASE_URL=postgres://... cargo test -p observability --test lifecycle_events postgres_repository -- --ignored`
#[actix_web::test]
#[ignore = "requires OBSERVABILITY_TEST_DATABASE_URL and applied observability migrations"]
async fn postgres_repository_preserves_versions_retention_boundary_and_atomicity() {
    const BOUNDARY_KEY: &str = "00000000000000000000000000000001";
    const EXPIRED_KEY: &str = "00000000000000000000000000000002";
    const DELIVERY_KEY: &str = "00000000000000000000000000000003";
    const ROLLED_BACK_KEY: &str = "00000000000000000000000000000004";

    let database_url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .expect("OBSERVABILITY_TEST_DATABASE_URL is required");
    let database = postgres_database(&database_url);
    let mut assertion_connection = PgConnection::establish(&database_url).unwrap();
    diesel::delete(alert_lifecycle_events::table)
        .execute(&mut assertion_connection)
        .unwrap();
    let store = observability::db::Store::new(&database).await.unwrap();
    let cutoff = datetime!(2026-06-01 0:00);

    store
        .replace_lifecycle_events(lifecycle_events::LifecycleEventsBatch {
            events: vec![
                repository_event(BOUNDARY_KEY, cutoff),
                repository_event(EXPIRED_KEY, cutoff - Duration::seconds(1)),
            ],
            retention_cutoff: cutoff,
        })
        .await
        .unwrap();
    let retained_keys = alert_lifecycle_events::table
        .select(alert_lifecycle_events::alert_key)
        .order(alert_lifecycle_events::alert_key)
        .load::<String>(&mut assertion_connection)
        .unwrap();
    assert_eq!(retained_keys, [BOUNDARY_KEY]);

    let mut delivered = repository_event(DELIVERY_KEY, cutoff + Duration::seconds(2));
    delivered.sent = true;
    delivered.ts_slack = "1712345.678".to_owned();
    store
        .replace_lifecycle_events(lifecycle_events::LifecycleEventsBatch {
            events: vec![delivered],
            retention_cutoff: cutoff,
        })
        .await
        .unwrap();
    store
        .replace_lifecycle_events(lifecycle_events::LifecycleEventsBatch {
            events: vec![repository_event(
                DELIVERY_KEY,
                cutoff + Duration::seconds(1),
            )],
            retention_cutoff: cutoff,
        })
        .await
        .unwrap();
    let delivery = alert_lifecycle_events::table
        .find(DELIVERY_KEY)
        .select(StorageLifecycleEvent::as_select())
        .first::<StorageLifecycleEvent>(&mut assertion_connection)
        .unwrap();
    assert!(delivery.sent);
    assert_eq!(delivery.ts_slack, "1712345.678");
    assert_eq!(delivery.last_updated_at, cutoff + Duration::seconds(2));

    let transaction_result = store
        .replace_lifecycle_events(lifecycle_events::LifecycleEventsBatch {
            events: vec![
                repository_event(ROLLED_BACK_KEY, cutoff + Duration::seconds(3)),
                repository_event(
                    "000000000000000000000000000000000",
                    cutoff + Duration::seconds(3),
                ),
            ],
            retention_cutoff: cutoff + Duration::seconds(1),
        })
        .await;
    assert!(transaction_result.is_err());
    assert_eq!(
        alert_lifecycle_events::table
            .filter(alert_lifecycle_events::alert_key.eq(ROLLED_BACK_KEY))
            .count()
            .get_result::<i64>(&mut assertion_connection)
            .unwrap(),
        0
    );
    assert_eq!(
        alert_lifecycle_events::table
            .filter(alert_lifecycle_events::alert_key.eq(BOUNDARY_KEY))
            .count()
            .get_result::<i64>(&mut assertion_connection)
            .unwrap(),
        1,
        "cleanup must roll back with the failed multi-row batch"
    );
}
