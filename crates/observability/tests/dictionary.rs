//! Alert dictionary tests through the authenticated Actix route tree.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::{Arc, Mutex};

use actix_web::{http::StatusCode, test, App};
use diesel::{Connection, PgConnection, RunQueryDsl};
use diesel_models::{
    errors::DatabaseError, observability::schema::alert_dictionary, StorageResult,
};
use error_stack::report;
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::{
        alerts_info::AlertsInfoInterface, blacklist::BlacklistInterface,
        dictionary::DictionaryInterface, metadata::AlertMetadataInterface,
        rule_toggles::RuleTogglesInterface, thresholds::ThresholdsInterface, StorageInterface,
    },
    domain::notifier::Registry,
    domain_models::{alerts_info, blacklist, dictionary, metadata, rule_toggles, thresholds},
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};
use time::{macros::datetime, PrimitiveDateTime};

const API_KEY: &str = "dictionary_test_key";

#[derive(Default)]
struct MemoryStore {
    rows: Mutex<Vec<dictionary::DictionaryEntry>>,
}

struct FailingStore;

macro_rules! impl_unused_metadata {
    ($store:ty) => {
        #[async_trait::async_trait]
        impl AlertMetadataInterface for $store {
            async fn list_alert_metadata(
                &self,
            ) -> StorageResult<Vec<metadata::AlertMetadataEntry>> {
                Err(report!(DatabaseError::Others))
            }

            async fn patch_alert_metadata(
                &self,
                _patch: metadata::AlertMetadataPatch,
            ) -> StorageResult<metadata::AlertMetadataEntry> {
                Err(report!(DatabaseError::Others))
            }
        }
    };
}

impl_unused_metadata!(MemoryStore);
impl_unused_metadata!(FailingStore);

#[async_trait::async_trait]
impl observability::db::lifecycle_events::LifecycleEventsInterface for MemoryStore {
    async fn list_lifecycle_events(
        &self,
        _from: Option<PrimitiveDateTime>,
        _to: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<observability::domain_models::lifecycle_events::LifecycleEvent>> {
        Err(report!(DatabaseError::Others))
    }

    async fn replace_lifecycle_events(
        &self,
        _batch: observability::domain_models::lifecycle_events::LifecycleEventsBatch,
    ) -> StorageResult<usize> {
        Err(report!(DatabaseError::Others))
    }
}

impl StorageInterface for MemoryStore {}
#[async_trait::async_trait]
impl observability::db::lifecycle_events::LifecycleEventsInterface for FailingStore {
    async fn list_lifecycle_events(
        &self,
        _from: Option<PrimitiveDateTime>,
        _to: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<observability::domain_models::lifecycle_events::LifecycleEvent>> {
        Err(report!(DatabaseError::Others))
    }

    async fn replace_lifecycle_events(
        &self,
        _batch: observability::domain_models::lifecycle_events::LifecycleEventsBatch,
    ) -> StorageResult<usize> {
        Err(report!(DatabaseError::Others))
    }
}

impl StorageInterface for FailingStore {}

fn now() -> PrimitiveDateTime {
    datetime!(2026-09-15 09:45:00)
}

macro_rules! impl_unused_interfaces {
    ($store:ty) => {
        #[async_trait::async_trait]
        impl AlertsInfoInterface for $store {
            async fn insert_alert_info(
                &self,
                _new: alerts_info::AlertsInfoNew,
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
                _new: blacklist::BlacklistEntryNew,
                _max_active_rules: i64,
            ) -> StorageResult<blacklist::BlacklistUpsertOutcome> {
                Err(report!(DatabaseError::Others))
            }

            async fn delete_blacklist_entry(
                &self,
                _new: blacklist::BlacklistEntryNew,
            ) -> StorageResult<blacklist::BlacklistEntry> {
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
                _new: rule_toggles::RuleToggleNew,
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
    };
}

impl_unused_interfaces!(MemoryStore);
impl_unused_interfaces!(FailingStore);

#[async_trait::async_trait]
impl DictionaryInterface for MemoryStore {
    async fn list_dictionary_entries(&self) -> StorageResult<Vec<dictionary::DictionaryEntry>> {
        let mut rows = self.rows.lock().unwrap().clone();
        rows.sort_by(|a, b| (&a.name, &a.key).cmp(&(&b.name, &b.key)));
        Ok(rows)
    }

    async fn upsert_dictionary_entry(
        &self,
        new: dictionary::DictionaryEntryNew,
    ) -> StorageResult<dictionary::DictionaryEntry> {
        let mut rows = self.rows.lock().unwrap();
        let stored = dictionary::DictionaryEntry {
            name: new.name,
            key: new.key,
            product: new.product,
            values: new.values,
            metadata: new.metadata,
            updated_by: new.updated_by,
            last_updated_at: now(),
        };
        if let Some(row) = rows
            .iter_mut()
            .find(|row| row.name == stored.name && row.key == stored.key)
        {
            *row = stored.clone();
        } else {
            rows.push(stored.clone());
        }
        Ok(stored)
    }
}

#[async_trait::async_trait]
impl DictionaryInterface for FailingStore {
    async fn list_dictionary_entries(&self) -> StorageResult<Vec<dictionary::DictionaryEntry>> {
        Err(report!(DatabaseError::Others))
    }

    async fn upsert_dictionary_entry(
        &self,
        _new: dictionary::DictionaryEntryNew,
    ) -> StorageResult<dictionary::DictionaryEntry> {
        Err(report!(DatabaseError::Others))
    }
}

fn state_with_store(store: Arc<dyn StorageInterface>) -> AppState {
    let conf: observability::Settings = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY }
    }))
    .unwrap();
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

async fn call(
    state: AppState,
    method: actix_web::http::Method,
    key: Option<&str>,
    payload: Option<Value>,
) -> (StatusCode, Value) {
    let app = test::init_service(App::new().service(Alerts::server(state))).await;
    let mut request = test::TestRequest::default()
        .method(method)
        .uri("/alerts/dictionary");
    if let Some(key) = key {
        request = request.insert_header((X_INTERNAL_API_KEY, key));
    }
    if let Some(payload) = payload {
        request = request.set_json(payload);
    }
    let response = test::call_service(&app, request.to_request()).await;
    let status = response.status();
    let bytes = test::read_body(response).await;
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

#[actix_web::test]
async fn upsert_replaces_strings_verbatim_and_preserves_response_shape() {
    let state = state();
    let first = json!({
        "name": "dashboard",
        "key_": "merchant_id",
        "product": "[]",
        "values_": "[\"m1\"]",
        "metadata": "{\"category\":\"dashboard\"}",
        "updated_by": "dashboard"
    });
    let (status, response) = call(
        state.clone(),
        actix_web::http::Method::PUT,
        Some(API_KEY),
        Some(first),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!({"ok": true, "name": "dashboard", "key_": "merchant_id"})
    );

    let replacement = json!({
        "name": "dashboard",
        "key_": "merchant_id",
        "product": "[\"payments\"]",
        "values_": "[\"m2\"]",
        "metadata": "{ \"encoded\": true }",
        "updated_by": "syntest"
    });
    call(
        state.clone(),
        actix_web::http::Method::PUT,
        Some(API_KEY),
        Some(replacement),
    )
    .await;

    let (status, listed) = call(state, actix_web::http::Method::GET, Some(API_KEY), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["entries"].as_array().unwrap().len(), 1);
    assert_eq!(listed["entries"][0]["key_"], "merchant_id");
    assert_eq!(listed["entries"][0]["values_"], "[\"m2\"]");
    assert!(listed["entries"][0].get("key").is_none());
    assert!(listed["entries"][0].get("values").is_none());
    assert_eq!(listed["entries"][0]["metadata"], "{ \"encoded\": true }");
    assert_eq!(listed["entries"][0]["updated_by"], "syntest");
    assert_eq!(
        listed["entries"][0]["last_updated_at"],
        "2026-09-15T09:45:00.000Z"
    );
}

#[actix_web::test]
async fn omitted_payload_strings_replace_existing_values_with_defaults() {
    let state = state();
    call(
        state.clone(),
        actix_web::http::Method::PUT,
        Some(API_KEY),
        Some(json!({
            "name": "dashboard", "key_": "merchant_id", "product": "[1]",
            "values_": "[2]", "metadata": "{\"a\":1}", "updated_by": "dashboard"
        })),
    )
    .await;
    call(
        state.clone(),
        actix_web::http::Method::PUT,
        Some(API_KEY),
        Some(json!({
            "name": "dashboard", "key_": "merchant_id", "updated_by": "dashboard"
        })),
    )
    .await;
    let (_, listed) = call(state, actix_web::http::Method::GET, Some(API_KEY), None).await;
    assert_eq!(listed["entries"][0]["product"], "[]");
    assert_eq!(listed["entries"][0]["values_"], "[]");
    assert_eq!(listed["entries"][0]["metadata"], "{}");
}

#[actix_web::test]
async fn entries_are_ordered_by_name_and_key() {
    let state = state();
    for (name, key) in [("z", "a"), ("a", "z"), ("a", "a")] {
        call(
            state.clone(),
            actix_web::http::Method::PUT,
            Some(API_KEY),
            Some(json!({"name": name, "key_": key, "updated_by": "dashboard"})),
        )
        .await;
    }
    let (_, listed) = call(state, actix_web::http::Method::GET, Some(API_KEY), None).await;
    let keys: Vec<_> = listed["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["name"].as_str().unwrap(), row["key_"].as_str().unwrap()))
        .collect();
    assert_eq!(keys, vec![("a", "a"), ("a", "z"), ("z", "a")]);
}

#[actix_web::test]
async fn authentication_validation_and_unknown_fields_are_enforced() {
    for method in [actix_web::http::Method::GET, actix_web::http::Method::PUT] {
        let payload = (method == actix_web::http::Method::PUT)
            .then(|| json!({"name": "a", "key_": "b", "updated_by": "dashboard"}));
        for key in [None, Some("wrong")] {
            assert_eq!(
                call(state(), method.clone(), key, payload.clone()).await.0,
                StatusCode::UNAUTHORIZED
            );
        }
    }

    for payload in [
        json!({"name": "", "key_": "b", "updated_by": "dashboard"}),
        json!({"name": "a", "key_": "   ", "updated_by": "dashboard"}),
        json!({"name": "a", "key_": "b", "updated_by": "dashboard", "extra": true}),
        json!({"name": "a", "updated_by": "dashboard"}),
        json!({"name": "a", "key": "b", "updated_by": "dashboard"}),
        json!({"name": "a", "key_": "b", "values": "[]", "updated_by": "dashboard"}),
        json!({"name": "a", "key_": "b", "values_": ["m1"], "updated_by": "dashboard"}),
    ] {
        let (status, response) = call(
            state(),
            actix_web::http::Method::PUT,
            Some(API_KEY),
            Some(payload),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(response["error"]["code"], "IR_04");
    }
}

#[actix_web::test]
async fn storage_failures_return_500() {
    let state = state_with_store(Arc::new(FailingStore));
    for (method, payload) in [
        (actix_web::http::Method::GET, None),
        (
            actix_web::http::Method::PUT,
            Some(json!({"name": "a", "key_": "b", "updated_by": "dashboard"})),
        ),
    ] {
        let (status, response) = call(state.clone(), method, Some(API_KEY), payload).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response["error"]["type"], "observability_error");
    }
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
        pool_size: 2,
        min_idle_pool_size: 1,
        connection_timeout: 10,
    }
}

/// Run after applying the observability migrations:
/// `OBSERVABILITY_TEST_DATABASE_URL=postgres://... cargo test -p observability --test dictionary postgres_repository -- --ignored`
#[actix_web::test]
#[ignore = "requires OBSERVABILITY_TEST_DATABASE_URL and applied observability migrations"]
async fn postgres_repository_replaces_and_lists_dictionary_entries() {
    let database_url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .expect("OBSERVABILITY_TEST_DATABASE_URL is required");
    let database = postgres_database(&database_url);
    let mut connection = PgConnection::establish(&database_url).unwrap();
    diesel::delete(alert_dictionary::table)
        .execute(&mut connection)
        .unwrap();

    let state = state_with_store(Arc::new(
        observability::db::Store::new(&database).await.unwrap(),
    ));
    for values in ["[\"m1\"]", "[\"m2\"]"] {
        assert_eq!(
            call(
                state.clone(),
                actix_web::http::Method::PUT,
                Some(API_KEY),
                Some(json!({
                    "name": "dashboard", "key_": "merchant_id", "values_": values,
                    "updated_by": "database-test"
                })),
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let (_, listed) = call(state, actix_web::http::Method::GET, Some(API_KEY), None).await;
    assert_eq!(listed["entries"].as_array().unwrap().len(), 1);
    assert_eq!(listed["entries"][0]["values_"], "[\"m2\"]");
}
