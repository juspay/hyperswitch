//! Threshold resource tests through the real authenticated Actix route tree.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::sync::{Arc, Mutex};

use actix_web::{http::StatusCode, test, App};
use diesel::{
    Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};
use diesel_models::{
    errors::DatabaseError,
    observability::{
        alert_manager::thresholds::ThresholdOverride, schema::success_rate_threshold_overrides,
    },
    StorageResult,
};
use error_stack::report;
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::{
        alerts_info::AlertsInfoInterface, blacklist::BlacklistInterface,
        rule_toggles::RuleTogglesInterface, thresholds::ThresholdsInterface, StorageInterface,
    },
    domain::notifier::Registry,
    domain_models::{alerts_info, blacklist, rule_toggles, thresholds},
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};
use time::{macros::datetime, PrimitiveDateTime};

const API_KEY: &str = "threshold_test_key";

#[derive(Default)]
struct MemoryStore {
    rows: Mutex<Vec<thresholds::ThresholdOverride>>,
    max_active_rules: Option<i64>,
}

struct FailingStore;

#[async_trait::async_trait]
impl AlertsInfoInterface for MemoryStore {
    async fn insert_alert_info(
        &self,
        _new: alerts_info::AlertsInfoNew,
    ) -> StorageResult<alerts_info::AlertsInfo> {
        Err(report!(DatabaseError::Others))
    }
}

fn now() -> PrimitiveDateTime {
    datetime!(2026-09-09 12:34:56)
}

impl StorageInterface for MemoryStore {}
impl StorageInterface for FailingStore {}

macro_rules! impl_unused_blacklist {
    ($store:ty) => {
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
    };
}

impl_unused_blacklist!(MemoryStore);
impl_unused_blacklist!(FailingStore);

macro_rules! impl_unused_rule_toggles {
    ($store:ty) => {
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
    };
}

impl_unused_rule_toggles!(MemoryStore);
impl_unused_rule_toggles!(FailingStore);

#[async_trait::async_trait]
impl AlertsInfoInterface for FailingStore {
    async fn insert_alert_info(
        &self,
        _new: alerts_info::AlertsInfoNew,
    ) -> StorageResult<alerts_info::AlertsInfo> {
        Err(report!(DatabaseError::Others))
    }
}

#[async_trait::async_trait]
impl ThresholdsInterface for FailingStore {
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

#[async_trait::async_trait]
impl ThresholdsInterface for MemoryStore {
    async fn list_threshold_overrides(&self) -> StorageResult<Vec<thresholds::ThresholdOverride>> {
        let mut rows = self.rows.lock().unwrap().clone();
        rows.retain(|row| !row.is_deleted);
        rows.sort_by(|a, b| {
            (&a.merchant_id, &a.profile_id, &a.name, &a.product).cmp(&(
                &b.merchant_id,
                &b.profile_id,
                &b.name,
                &b.product,
            ))
        });
        Ok(rows)
    }

    async fn upsert_threshold_override(
        &self,
        new: thresholds::ThresholdOverrideNew,
        max_active_rules: i64,
    ) -> StorageResult<thresholds::ThresholdUpsertOutcome> {
        let max_active_rules = self.max_active_rules.unwrap_or(max_active_rules);
        let mut rows = self.rows.lock().unwrap();
        let existing = rows.iter().position(|row| {
            (&row.name, &row.product, &row.merchant_id, &row.profile_id)
                == (&new.name, &new.product, &new.merchant_id, &new.profile_id)
        });
        let already_active = existing.is_some_and(|index| !rows[index].is_deleted);
        let active_count =
            i64::try_from(rows.iter().filter(|row| !row.is_deleted).count()).unwrap();
        if max_active_rules > 0 && !already_active && active_count >= max_active_rules {
            return Ok(thresholds::ThresholdUpsertOutcome::ActiveRuleLimitReached);
        }

        let stored = thresholds::ThresholdOverride {
            name: new.name,
            product: new.product,
            merchant_id: new.merchant_id,
            profile_id: new.profile_id,
            min_volume: new.min_volume,
            min_impacted_volume: new.min_impacted_volume,
            tolerance: new.tolerance,
            diff_threshold: new.diff_threshold,
            updated_by: new.updated_by,
            last_updated_at: now(),
            is_deleted: false,
        };
        if let Some(index) = existing {
            rows[index] = stored.clone();
        } else {
            rows.push(stored.clone());
        }
        Ok(thresholds::ThresholdUpsertOutcome::Stored(stored))
    }

    async fn delete_threshold_override(
        &self,
        new: thresholds::ThresholdOverrideNew,
    ) -> StorageResult<thresholds::ThresholdOverride> {
        let mut rows = self.rows.lock().unwrap();
        let tombstone = thresholds::ThresholdOverride {
            name: new.name,
            product: new.product,
            merchant_id: new.merchant_id,
            profile_id: new.profile_id,
            min_volume: None,
            min_impacted_volume: None,
            tolerance: None,
            diff_threshold: None,
            updated_by: new.updated_by,
            last_updated_at: now(),
            is_deleted: true,
        };
        if let Some(row) = rows.iter_mut().find(|row| {
            (&row.name, &row.product, &row.merchant_id, &row.profile_id)
                == (
                    &tombstone.name,
                    &tombstone.product,
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

fn state(max_active_rules: i64) -> AppState {
    state_with_store(Arc::new(MemoryStore {
        rows: Mutex::new(Vec::new()),
        max_active_rules: Some(max_active_rules),
    }))
}

fn body() -> Value {
    json!({
        "name": "Success-rate drop",
        "product": "payments",
        "merchant_id": " merchant_123 ",
        "profile_id": " profile_123 ",
        "min_volume": 30.0,
        "min_impacted_volume": null,
        "tolerance": -3.0,
        "diff_threshold": 50.0,
        "updated_by": "dashboard"
    })
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
        .uri("/alerts/thresholds");
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
    let bytes = test::read_body(response).await;
    (status, serde_json::from_slice(&bytes).unwrap())
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

#[actix_web::test]
async fn all_methods_require_the_internal_key() {
    for method in [
        actix_web::http::Method::GET,
        actix_web::http::Method::POST,
        actix_web::http::Method::DELETE,
    ] {
        for key in [None, Some("wrong")] {
            let payload = if method == actix_web::http::Method::POST {
                Some(body())
            } else if method == actix_web::http::Method::DELETE {
                Some(json!({
                    "name": "Success-rate drop", "product": "payments",
                    "merchant_id": "merchant_123", "updated_by": "dashboard"
                }))
            } else {
                None
            };
            let (status, response) = call(state(5000), method.clone(), key, payload).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(response["error"]["code"], "IR_01");
        }
    }
}

#[actix_web::test]
async fn malformed_json_uses_the_standard_error_response() {
    for method in [
        actix_web::http::Method::POST,
        actix_web::http::Method::DELETE,
    ] {
        for key in [None, Some("wrong"), Some(API_KEY)] {
            let (status, response) =
                call_raw(state(5000), method.clone(), key, Some(b"{not-json")).await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(response["error"]["code"], "IR_04");
        }
    }
}

#[actix_web::test]
async fn upsert_list_and_delete_preserve_the_resource_contract() {
    let state = state(5000);
    let (status, created) = call(
        state.clone(),
        actix_web::http::Method::POST,
        Some(API_KEY),
        Some(body()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(created["threshold"]["merchant_id"], "merchant_123");
    assert_eq!(created["threshold"]["profile_id"], "profile_123");
    assert_eq!(created["threshold"]["min_impacted_volume"], Value::Null);
    assert_eq!(
        created["threshold"]["last_updated_at"],
        "2026-09-09T12:34:56.000Z"
    );

    let (_, listed) = call(
        state.clone(),
        actix_web::http::Method::GET,
        Some(API_KEY),
        None,
    )
    .await;
    assert_eq!(listed["overrides"].as_array().unwrap().len(), 1);

    let delete = json!({
        "name": "Success-rate drop", "product": "payments",
        "merchant_id": "merchant_123", "profile_id": "profile_123",
        "updated_by": "operator"
    });
    for _ in 0..2 {
        let (status, response) = call(
            state.clone(),
            actix_web::http::Method::DELETE,
            Some(API_KEY),
            Some(delete.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(response, json!({ "status": "deleted" }));
    }
    let (_, listed) = call(state, actix_web::http::Method::GET, Some(API_KEY), None).await;
    assert!(listed["overrides"].as_array().unwrap().is_empty());
}

#[actix_web::test]
async fn cap_returns_429_but_an_active_update_and_delete_succeed() {
    let state = state(1);
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body())
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
            Some(body())
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut second = body();
    second["merchant_id"] = json!("merchant_456");
    let (status, response) = call(
        state.clone(),
        actix_web::http::Method::POST,
        Some(API_KEY),
        Some(second),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(response["error"]["type"], "invalid_request");

    let delete = json!({
        "name": "Success-rate drop", "product": "payments",
        "merchant_id": "merchant_123", "updated_by": "dashboard"
    });
    assert_eq!(
        call(
            state,
            actix_web::http::Method::DELETE,
            Some(API_KEY),
            Some(delete)
        )
        .await
        .0,
        StatusCode::OK
    );
}

#[actix_web::test]
async fn replacement_nullability_resurrection_and_ordering_are_preserved() {
    let state = state(10);
    let mut first = body();
    first["merchant_id"] = json!("merchant_b");
    first["profile_id"] = json!("");
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(first.clone()),
        )
        .await
        .0,
        StatusCode::OK
    );

    // Omitted option fields have the same replacement meaning as explicit null.
    let replacement = json!({
        "name": "Success-rate drop", "product": "payments",
        "merchant_id": "merchant_b", "updated_by": "replacement"
    });
    let (_, updated) = call(
        state.clone(),
        actix_web::http::Method::POST,
        Some(API_KEY),
        Some(replacement),
    )
    .await;
    for field in [
        "min_volume",
        "min_impacted_volume",
        "tolerance",
        "diff_threshold",
    ] {
        assert_eq!(updated["threshold"][field], Value::Null);
    }
    assert_eq!(updated["threshold"]["updated_by"], "replacement");

    let delete = json!({
        "name": "Success-rate drop", "product": "payments",
        "merchant_id": "merchant_b", "updated_by": "deleter"
    });
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::DELETE,
            Some(API_KEY),
            Some(delete),
        )
        .await
        .0,
        StatusCode::OK
    );

    let mut resurrected = body();
    resurrected["merchant_id"] = json!("merchant_b");
    resurrected["profile_id"] = json!("");
    resurrected["min_volume"] = json!(99.0);
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(resurrected),
        )
        .await
        .0,
        StatusCode::OK
    );

    for (merchant, name, product) in [
        ("merchant_a", "Zero success", "payments"),
        ("merchant_a", "Success-rate drop", "refunds"),
    ] {
        let mut row = body();
        row["merchant_id"] = json!(merchant);
        row["profile_id"] = json!("");
        row["name"] = json!(name);
        row["product"] = json!(product);
        assert_eq!(
            call(
                state.clone(),
                actix_web::http::Method::POST,
                Some(API_KEY),
                Some(row),
            )
            .await
            .0,
            StatusCode::OK
        );
    }

    let (_, listed) = call(state, actix_web::http::Method::GET, Some(API_KEY), None).await;
    let keys: Vec<_> = listed["overrides"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["merchant_id"].as_str().unwrap(),
                row["name"].as_str().unwrap(),
                row["product"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        keys,
        vec![
            ("merchant_a", "Success-rate drop", "refunds"),
            ("merchant_a", "Zero success", "payments"),
            ("merchant_b", "Success-rate drop", "payments"),
        ]
    );
}

#[actix_web::test]
async fn tombstones_preserve_delete_attribution_and_null_every_value() {
    let store = Arc::new(MemoryStore::default());
    let state = state_with_store(store.clone());
    let delete = json!({
        "name": "Success-rate drop", "product": "payments",
        "merchant_id": "unknown", "updated_by": "operator"
    });
    assert_eq!(
        call(
            state,
            actix_web::http::Method::DELETE,
            Some(API_KEY),
            Some(delete),
        )
        .await
        .0,
        StatusCode::OK
    );

    let rows = store.rows.lock().unwrap();
    let tombstone = &rows[0];
    assert!(tombstone.is_deleted);
    assert_eq!(tombstone.updated_by, "operator");
    assert_eq!(tombstone.last_updated_at, now());
    assert_eq!(
        (
            tombstone.min_volume,
            tombstone.min_impacted_volume,
            tombstone.tolerance,
            tombstone.diff_threshold,
        ),
        (None, None, None, None)
    );
}

#[actix_web::test]
async fn resurrection_is_rejected_at_cap_and_non_positive_caps_are_disabled() {
    let capped_state = state(1);
    let delete = json!({
        "name": "Success-rate drop", "product": "payments",
        "merchant_id": "merchant_123", "profile_id": "profile_123",
        "updated_by": "operator"
    });
    assert_eq!(
        call(
            capped_state.clone(),
            actix_web::http::Method::DELETE,
            Some(API_KEY),
            Some(delete),
        )
        .await
        .0,
        StatusCode::OK
    );
    let mut active = body();
    active["merchant_id"] = json!("other");
    assert_eq!(
        call(
            capped_state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(active),
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            capped_state,
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(body()),
        )
        .await
        .0,
        StatusCode::TOO_MANY_REQUESTS
    );

    for cap in [0, -1] {
        let state = state(cap);
        for merchant in ["one", "two"] {
            let mut row = body();
            row["merchant_id"] = json!(merchant);
            assert_eq!(
                call(
                    state.clone(),
                    actix_web::http::Method::POST,
                    Some(API_KEY),
                    Some(row),
                )
                .await
                .0,
                StatusCode::OK
            );
        }
    }
}

#[actix_web::test]
async fn storage_failures_return_500_for_every_method() {
    let state = state_with_store(Arc::new(FailingStore));
    for (method, payload) in [
        (actix_web::http::Method::GET, None),
        (actix_web::http::Method::POST, Some(body())),
        (
            actix_web::http::Method::DELETE,
            Some(json!({
                "name": "name", "product": "product", "merchant_id": "merchant",
                "updated_by": "dashboard"
            })),
        ),
    ] {
        let (status, response) = call(state.clone(), method, Some(API_KEY), payload).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response["error"]["type"], "observability_error");
    }
}

#[actix_web::test]
async fn invalid_scope_actor_and_missing_keys_return_400() {
    for patch in [("merchant_id", json!("  ")), ("updated_by", json!(""))] {
        let mut payload = body();
        payload[patch.0] = patch.1;
        let (status, response) = call(
            state(5000),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(payload),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(response["error"]["code"], "IR_04");
    }

    let (status, response) = call(
        state(5000),
        actix_web::http::Method::POST,
        Some(API_KEY),
        Some(json!({ "merchant_id": "merchant", "updated_by": "dashboard" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(response["error"]["code"], "IR_04");
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

/// Exercises the actual Diesel repository. Run after applying the observability migrations:
///
/// `OBSERVABILITY_TEST_DATABASE_URL=postgres://... cargo test -p observability --test thresholds postgres_repository -- --ignored`
#[actix_web::test]
#[ignore = "requires OBSERVABILITY_TEST_DATABASE_URL and applied observability migrations"]
async fn postgres_repository_enforces_replacement_tombstones_and_ordering() {
    let database_url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .expect("OBSERVABILITY_TEST_DATABASE_URL is required");
    let database = postgres_database(&database_url);
    let mut assertion_connection = PgConnection::establish(&database_url).unwrap();
    diesel::delete(success_rate_threshold_overrides::table)
        .execute(&mut assertion_connection)
        .unwrap();

    let store = Arc::new(observability::db::Store::new(&database).await.unwrap());
    let state = state_with_store(store);

    // An all-null active row is valid and consumes one slot.
    let all_null = json!({
        "name": "Zero success", "product": "payments", "merchant_id": "merchant_b",
        "updated_by": "creator"
    });
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(all_null),
        )
        .await
        .0,
        StatusCode::OK
    );

    let mut first = body();
    first["merchant_id"] = json!("merchant_a");
    first["profile_id"] = json!("");
    assert_eq!(
        call(
            state.clone(),
            actix_web::http::Method::POST,
            Some(API_KEY),
            Some(first.clone()),
        )
        .await
        .0,
        StatusCode::OK
    );

    // Replacement at capacity succeeds and replaces values rather than patching them.
    first["min_volume"] = json!(71.0);
    first.as_object_mut().unwrap().remove("tolerance");
    let (_, updated) = call(
        state.clone(),
        actix_web::http::Method::POST,
        Some(API_KEY),
        Some(first),
    )
    .await;
    assert_eq!(updated["threshold"]["min_volume"], 71.0);
    assert_eq!(updated["threshold"]["tolerance"], Value::Null);

    // Deleting at capacity succeeds and leaves an attributed, fully-null tombstone.
    let delete = json!({
        "name": "Success-rate drop", "product": "payments", "merchant_id": "merchant_a",
        "updated_by": "database-test-deleter"
    });
    for _ in 0..2 {
        assert_eq!(
            call(
                state.clone(),
                actix_web::http::Method::DELETE,
                Some(API_KEY),
                Some(delete.clone()),
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let tombstone = success_rate_threshold_overrides::table
        .filter(success_rate_threshold_overrides::merchant_id.eq("merchant_a"))
        .select(ThresholdOverride::as_select())
        .first::<ThresholdOverride>(&mut assertion_connection)
        .unwrap();
    assert!(tombstone.is_deleted);
    assert_eq!(tombstone.updated_by, "database-test-deleter");
    assert_eq!(
        (
            tombstone.min_volume,
            tombstone.min_impacted_volume,
            tombstone.tolerance,
            tombstone.diff_threshold,
        ),
        (None, None, None, None)
    );

    let unknown_delete = json!({
        "name": "Zero success", "product": "payments", "merchant_id": "never-created",
        "updated_by": "database-test-deleter"
    });
    for _ in 0..2 {
        assert_eq!(
            call(
                state.clone(),
                actix_web::http::Method::DELETE,
                Some(API_KEY),
                Some(unknown_delete.clone()),
            )
            .await
            .0,
            StatusCode::OK
        );
    }

    let (_, listed) = call(state, actix_web::http::Method::GET, Some(API_KEY), None).await;
    let merchants: Vec<_> = listed["overrides"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["merchant_id"].as_str().unwrap())
        .collect();
    assert_eq!(merchants, ["merchant_b"]);
}
