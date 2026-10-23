//! Alert rule tests through the authenticated Actix route tree.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{
    http::{Method, StatusCode},
    test, App,
};
use common_types::alert_rules::stored::AlertRuleStatus;
use diesel::{Connection, PgConnection, RunQueryDsl};
use diesel_models::{
    errors::DatabaseError,
    observability::{alert_manager::alert_rules as storage, schema},
    StorageResult,
};
use error_stack::report;
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::{
        alert_rules::AlertRulesInterface, alerts_info::AlertsInfoInterface,
        blacklist::BlacklistInterface, dictionary::DictionaryInterface,
        lifecycle_events::LifecycleEventsInterface, metadata::AlertMetadataInterface,
        rule_toggles::RuleTogglesInterface, thresholds::ThresholdsInterface, StorageInterface,
    },
    domain::notifier::{
        chat::{ChatNotifier, LogChatNotifier},
        Registry,
    },
    domain_models::{
        alert_rules::{AlertRule, AlertRuleNew},
        alerts_info, blacklist, dictionary, lifecycle_events, metadata, rule_toggles, thresholds,
    },
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};
use time::{macros::datetime, PrimitiveDateTime};

const API_KEY: &str = "alert_rules_test_key";
const DESTINATION: &str = "infra_oncall";

/// When the memory store records a rule as created or changed.
const STORED_AT: PrimitiveDateTime = datetime!(2026-10-22 09:00:00);

#[derive(Default)]
struct MemoryStore {
    rules: Mutex<Vec<AlertRule>>,
}

#[async_trait::async_trait]
impl AlertRulesInterface for MemoryStore {
    async fn list_alert_rules(&self) -> StorageResult<Vec<AlertRule>> {
        let mut rules = self.rules.lock().unwrap().clone();
        rules.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(rules)
    }

    async fn find_alert_rule(&self, id: &str) -> StorageResult<Option<AlertRule>> {
        Ok(self
            .rules
            .lock()
            .unwrap()
            .iter()
            .find(|rule| rule.id == id)
            .cloned())
    }

    async fn upsert_alert_rule(&self, new: AlertRuleNew) -> StorageResult<AlertRule> {
        let new = storage::AlertRuleNew::from(new);
        let mut rules = self.rules.lock().unwrap();
        let created_at = rules
            .iter()
            .find(|rule| rule.id == new.id)
            .map_or(STORED_AT, |rule| rule.created_at);
        rules.retain(|rule| rule.id != new.id);
        let rule = AlertRule {
            id: new.id,
            name: new.name,
            status: new.status,
            alerts_configuration: new.alerts_configuration,
            next_run_at: new.next_run_at,
            created_at,
            last_updated_at: STORED_AT,
        };
        rules.push(rule.clone());
        Ok(rule)
    }

    async fn delete_alert_rule(&self, id: &str) -> StorageResult<bool> {
        let mut rules = self.rules.lock().unwrap();
        let before = rules.len();
        rules.retain(|rule| rule.id != id);
        Ok(rules.len() < before)
    }

    async fn list_active_alert_rules(&self) -> StorageResult<Vec<AlertRule>> {
        Ok(self
            .list_alert_rules()
            .await?
            .into_iter()
            .filter(|rule| rule.status == AlertRuleStatus::Active)
            .collect())
    }

    async fn list_due_alert_rules(&self, now: PrimitiveDateTime) -> StorageResult<Vec<AlertRule>> {
        Ok(self
            .list_active_alert_rules()
            .await?
            .into_iter()
            .filter(|rule| rule.next_run_at <= now)
            .collect())
    }

    async fn claim_alert_rule(
        &self,
        id: &str,
        seen: PrimitiveDateTime,
        next: PrimitiveDateTime,
    ) -> StorageResult<bool> {
        let mut rules = self.rules.lock().unwrap();
        let claimed = rules.iter_mut().find(|rule| {
            rule.id == id && rule.status == AlertRuleStatus::Active && rule.next_run_at == seen
        });
        Ok(claimed.map(|rule| rule.next_run_at = next).is_some())
    }
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
impl BlacklistInterface for MemoryStore {
    async fn list_blacklist_entries(&self) -> StorageResult<Vec<blacklist::BlacklistEntry>> {
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
impl DictionaryInterface for MemoryStore {
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

#[async_trait::async_trait]
impl LifecycleEventsInterface for MemoryStore {
    async fn list_lifecycle_events(
        &self,
        _from: Option<PrimitiveDateTime>,
        _to: Option<PrimitiveDateTime>,
    ) -> StorageResult<Vec<lifecycle_events::LifecycleEvent>> {
        Err(report!(DatabaseError::Others))
    }

    async fn replace_lifecycle_events(
        &self,
        _batch: lifecycle_events::LifecycleEventsBatch,
    ) -> StorageResult<usize> {
        Err(report!(DatabaseError::Others))
    }
}

#[async_trait::async_trait]
impl AlertMetadataInterface for MemoryStore {
    async fn list_alert_metadata(&self) -> StorageResult<Vec<metadata::AlertMetadataEntry>> {
        Err(report!(DatabaseError::Others))
    }

    async fn patch_alert_metadata(
        &self,
        _patch: metadata::AlertMetadataPatch,
    ) -> StorageResult<metadata::AlertMetadataEntry> {
        Err(report!(DatabaseError::Others))
    }
}

#[async_trait::async_trait]
impl RuleTogglesInterface for MemoryStore {
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

/// `alert` is the `[alert]` settings section.
fn state_with(store: Arc<dyn StorageInterface>, alert: Value) -> AppState {
    let conf: observability::Settings = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY },
        "alert": alert
    }))
    .unwrap();
    let chat: Arc<dyn ChatNotifier> = Arc::new(LogChatNotifier::new(DESTINATION.to_owned()));
    AppState {
        router_transport: None,
        conf: Arc::new(conf),
        chat: Arc::new(Registry::new(HashMap::from([(
            DESTINATION.to_owned(),
            chat,
        )]))),
        email: Arc::new(Registry::default()),
        store,
    }
}

fn state() -> AppState {
    state_with(
        Arc::new(MemoryStore::default()),
        json!({ "datasources": { "grafana_cloudwatch": { "uid": "cloudwatch" } } }),
    )
}

/// A `PUT /alerts/rules/{id}` body.
fn rule(name: &str) -> Value {
    json!({
        "name": name,
        "status": "active",
        "alerts_configuration": {
            "version": "v1",
            "interval_seconds": 60,
            "query": {
                "datasource": "cloudwatch",
                "version": "v1",
                "region": "us-east-1",
                "namespace": "AWS/RDS",
                "metric_name": "CPUUtilization",
                "statistic": "Average",
                "period": 60
            },
            "thresholds": {
                "version": "v1",
                "severities": {
                    "critical": {
                        "threshold": 80.0,
                        "comparison_operator": "GreaterThanOrEqualToThreshold",
                        "description": "RDS writer CPU above 80%.",
                        "destination": DESTINATION
                    }
                }
            }
        }
    })
}

fn set(mut value: Value, pointer: &str, replacement: Value) -> Value {
    *value.pointer_mut(pointer).unwrap() = replacement;
    value
}

async fn call(
    state: AppState,
    method: Method,
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
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

#[actix_web::test]
async fn a_rule_is_created_read_listed_replaced_and_deleted() {
    let state = state();
    let uri = "/alerts/rules/rds-writer-cpu";

    let (status, created) = call(
        state.clone(),
        Method::PUT,
        uri,
        Some(API_KEY),
        Some(rule("RDS writer CPU")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(created["id"], "rds-writer-cpu");
    assert_eq!(created["name"], "RDS writer CPU");
    assert_eq!(created["status"], "active");
    // Left-out fields come back with their defaults.
    assert_eq!(
        created["alerts_configuration"]["query"]["end_offset_seconds"],
        180
    );
    assert_eq!(
        created["alerts_configuration"]["query"]["dimensions"],
        json!({})
    );
    assert!(created["next_run_at"].is_string());
    assert_eq!(created["created_at"], "2026-10-22T09:00:00.000Z");

    let (status, read) = call(state.clone(), Method::GET, uri, Some(API_KEY), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read, created);

    let replacement = set(
        rule("  RDS writer CPU (paused) "),
        "/status",
        json!("inactive"),
    );
    let (status, replaced) = call(
        state.clone(),
        Method::PUT,
        uri,
        Some(API_KEY),
        Some(replacement),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replaced["name"], "RDS writer CPU (paused)");
    assert_eq!(replaced["status"], "inactive");

    let (status, listed) = call(
        state.clone(),
        Method::GET,
        "/alerts/rules",
        Some(API_KEY),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed, json!({ "rules": [replaced] }));

    let (status, deleted) = call(state.clone(), Method::DELETE, uri, Some(API_KEY), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(deleted, json!({ "ok": true, "id": "rds-writer-cpu" }));

    for method in [Method::GET, Method::DELETE] {
        let (status, missing) = call(state.clone(), method, uri, Some(API_KEY), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(missing["error"]["code"], "IR_09");
    }
}

#[actix_web::test]
async fn every_route_needs_the_api_key() {
    for (method, uri, payload) in [
        (Method::GET, "/alerts/rules", None),
        (Method::GET, "/alerts/rules/rule", None),
        (Method::PUT, "/alerts/rules/rule", Some(rule("n"))),
        (Method::DELETE, "/alerts/rules/rule", None),
    ] {
        for key in [None, Some("wrong")] {
            let (status, _) = call(state(), method.clone(), uri, key, payload.clone()).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
        }
    }
}

#[actix_web::test]
async fn an_invalid_rule_is_rejected_with_its_reason() {
    let critical = "/alerts_configuration/thresholds/severities/critical";
    let cases = [
        ("/alerts/rules/RDS", rule("n"), "id must be"),
        ("/alerts/rules/rule", rule("  "), "name must be"),
        (
            "/alerts/rules/rule",
            set(
                rule("n"),
                "/alerts_configuration/interval_seconds",
                json!(90),
            ),
            "alerts_configuration.interval_seconds",
        ),
        (
            "/alerts/rules/rule",
            set(
                rule("n"),
                &format!("{critical}/destination"),
                json!("nowhere"),
            ),
            "alerts_configuration.thresholds.severities.critical.destination `nowhere` is not a \
             configured chat destination",
        ),
    ];

    let state = state();
    for (uri, payload, reason) in cases {
        let (status, response) = call(
            state.clone(),
            Method::PUT,
            uri,
            Some(API_KEY),
            Some(payload),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{reason}");
        assert_eq!(response["error"]["code"], "IR_04");
        let message = response["error"]["message"].as_str().unwrap();
        assert!(
            message.starts_with(reason),
            "expected `{reason}`: {message}"
        );
    }

    // A deployment that cannot query the rule's datasource does not accept it.
    let unconfigured = state_with(Arc::new(MemoryStore::default()), json!({}));
    let (status, response) = call(
        unconfigured,
        Method::PUT,
        "/alerts/rules/rule",
        Some(API_KEY),
        Some(rule("n")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("`alert.datasources.grafana_cloudwatch`"));

    // A body that does not parse gets the service's usual answer, which does not echo it.
    let (status, response) = call(
        state.clone(),
        Method::PUT,
        "/alerts/rules/rule",
        Some(API_KEY),
        Some(set(rule("n"), "/alerts_configuration/version", json!("v9"))),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        response["error"]["message"],
        "The request body could not be parsed"
    );

    let (_, listed) = call(state, Method::GET, "/alerts/rules", Some(API_KEY), None).await;
    assert_eq!(listed, json!({ "rules": [] }));
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
/// `OBSERVABILITY_TEST_DATABASE_URL=postgres://... cargo test -p observability --test alert_rules postgres_repository -- --ignored`
#[actix_web::test]
#[ignore = "requires OBSERVABILITY_TEST_DATABASE_URL and applied observability migrations"]
async fn postgres_repository_stores_replaces_claims_and_deletes_rules() {
    let database_url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .expect("OBSERVABILITY_TEST_DATABASE_URL is required");
    let mut connection = PgConnection::establish(&database_url).unwrap();
    diesel::delete(schema::alert_rules::table)
        .execute(&mut connection)
        .unwrap();

    let store = Arc::new(
        observability::db::Store::new(&postgres_database(&database_url))
            .await
            .unwrap(),
    );
    let state = state_with(
        store.clone(),
        json!({ "datasources": { "grafana_cloudwatch": { "uid": "cloudwatch" } } }),
    );
    let uri = "/alerts/rules/rds-writer-cpu";

    let (status, created) = call(
        state.clone(),
        Method::PUT,
        uri,
        Some(API_KEY),
        Some(rule("RDS writer CPU")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The stored configuration reads back as it was written.
    let (_, read) = call(state.clone(), Method::GET, uri, Some(API_KEY), None).await;
    assert_eq!(read, created);

    // Replacing keeps `created_at` and makes the rule due again.
    let (_, replaced) = call(
        state.clone(),
        Method::PUT,
        uri,
        Some(API_KEY),
        Some(rule("RDS writer CPU, renamed")),
    )
    .await;
    assert_eq!(replaced["created_at"], created["created_at"]);
    assert_eq!(replaced["name"], "RDS writer CPU, renamed");

    // Due once its time has come; a claim succeeds once for the time it was read at.
    let rule = store
        .find_alert_rule("rds-writer-cpu")
        .await
        .unwrap()
        .unwrap();
    let later = rule.next_run_at + time::Duration::minutes(1);
    assert!(store
        .list_due_alert_rules(rule.next_run_at - time::Duration::seconds(1))
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        store.list_due_alert_rules(later).await.unwrap(),
        [rule.clone()]
    );
    assert!(store
        .claim_alert_rule(&rule.id, rule.next_run_at, later)
        .await
        .unwrap());
    assert!(!store
        .claim_alert_rule(&rule.id, rule.next_run_at, later)
        .await
        .unwrap());
    assert!(store
        .list_due_alert_rules(later - time::Duration::seconds(1))
        .await
        .unwrap()
        .is_empty());

    // An inactive rule is neither active nor due, and cannot be claimed.
    call(
        state.clone(),
        Method::PUT,
        uri,
        Some(API_KEY),
        Some(set(self::rule("n"), "/status", json!("inactive"))),
    )
    .await;
    let inactive = store
        .find_alert_rule("rds-writer-cpu")
        .await
        .unwrap()
        .unwrap();
    assert!(store.list_active_alert_rules().await.unwrap().is_empty());
    assert!(store.list_due_alert_rules(later).await.unwrap().is_empty());
    assert!(!store
        .claim_alert_rule(&inactive.id, inactive.next_run_at, later)
        .await
        .unwrap());

    assert_eq!(
        call(state.clone(), Method::DELETE, uri, Some(API_KEY), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        call(state, Method::DELETE, uri, Some(API_KEY), None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}
