#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use actix_web::{http::StatusCode, test, App};
use diesel::{Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use diesel_models::{errors::DatabaseError, StorageResult};
use error_stack::report;
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::{alert_rule_configurations::AlertRuleConfigurationInterface, StorageInterface},
    domain::notifier::Registry,
    domain_models::alert_rule_configurations::{
        AlertRuleConfiguration, AlertRuleConfigurationListing, AlertRuleConfigurationNew,
        AlertRuleConfigurationUpdate,
    },
    routes::Alerts,
    settings::Database,
    state::AppState,
};
use serde_json::{json, Value};
use time::macros::datetime;

const API_KEY: &str = "alert_rule_configuration_test_key";
const COLLECTION_URI: &str = "/alerts/alerts_manager/rule_configurations";

#[derive(Default)]
struct MemoryStore {
    rows: Mutex<Vec<AlertRuleConfiguration>>,
}

struct FailingStore;

fn initial_time() -> time::PrimitiveDateTime {
    datetime!(2026-09-15 09:45:00)
}

fn updated_time() -> time::PrimitiveDateTime {
    datetime!(2026-09-15 09:46:00)
}

#[async_trait::async_trait]
impl AlertRuleConfigurationInterface for MemoryStore {
    async fn insert_alert_rule_configuration(
        &self,
        new: AlertRuleConfigurationNew,
    ) -> StorageResult<AlertRuleConfiguration> {
        let mut rows = self.rows.lock().unwrap();
        if rows
            .iter()
            .any(|row| row.id == new.id || row.name == new.name)
        {
            return Err(report!(DatabaseError::UniqueViolation));
        }
        let row = AlertRuleConfiguration {
            id: new.id,
            name: new.name,
            classification: new.classification,
            datasource: new.datasource,
            query: new.query,
            thresholds: new.thresholds,
            interval_seconds: new.interval_seconds,
            is_enabled: new.is_enabled,
            next_run_at: initial_time(),
            last_evaluated_at: None,
            created_at: initial_time(),
            last_updated_at: initial_time(),
        };
        rows.push(row.clone());
        Ok(row)
    }

    async fn update_alert_rule_configuration(
        &self,
        id: String,
        update: AlertRuleConfigurationUpdate,
    ) -> StorageResult<AlertRuleConfiguration> {
        let mut rows = self.rows.lock().unwrap();
        let row = rows
            .iter_mut()
            .find(|row| row.id == id)
            .ok_or_else(|| report!(DatabaseError::NotFound))?;
        match update {
            AlertRuleConfigurationUpdate::Update {
                name,
                classification,
                query,
                thresholds,
                interval_seconds,
                is_enabled,
            } => {
                if let Some(name) = name {
                    row.name = name;
                }
                if let Some(classification) = classification {
                    row.classification = classification;
                }
                if let Some(query) = query {
                    row.query = query;
                }
                if let Some(thresholds) = thresholds {
                    row.thresholds = thresholds;
                }
                if let Some(interval_seconds) = interval_seconds {
                    row.interval_seconds = interval_seconds;
                }
                if let Some(is_enabled) = is_enabled {
                    row.is_enabled = is_enabled;
                }
                row.last_updated_at = updated_time();
            }
            AlertRuleConfigurationUpdate::NextRun { next_run_at } => {
                row.next_run_at = next_run_at;
            }
            AlertRuleConfigurationUpdate::Evaluated { last_evaluated_at } => {
                row.last_evaluated_at = Some(last_evaluated_at);
            }
        }
        Ok(row.clone())
    }

    async fn list_alert_rule_configurations(&self) -> StorageResult<AlertRuleConfigurationListing> {
        let mut rules = self.rows.lock().unwrap().clone();
        rules.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(AlertRuleConfigurationListing {
            rules,
            unreadable_ids: Vec::new(),
        })
    }

    async fn list_due_alert_rule_configurations(
        &self,
        now: time::PrimitiveDateTime,
    ) -> StorageResult<AlertRuleConfigurationListing> {
        let mut rules: Vec<_> = self
            .rows
            .lock()
            .unwrap()
            .iter()
            .filter(|row| row.is_enabled && row.next_run_at <= now)
            .cloned()
            .collect();
        rules.sort_by_key(|row| row.next_run_at);
        Ok(AlertRuleConfigurationListing {
            rules,
            unreadable_ids: Vec::new(),
        })
    }

    async fn set_alert_rule_next_run_at(
        &self,
        id: String,
        next_run_at: time::PrimitiveDateTime,
    ) -> StorageResult<bool> {
        let mut rows = self.rows.lock().unwrap();
        let Some(row) = rows.iter_mut().find(|row| row.id == id) else {
            return Ok(false);
        };
        row.next_run_at = next_run_at;
        Ok(true)
    }

    async fn set_alert_rule_last_evaluated_at(
        &self,
        id: String,
        last_evaluated_at: time::PrimitiveDateTime,
    ) -> StorageResult<bool> {
        let mut rows = self.rows.lock().unwrap();
        let Some(row) = rows.iter_mut().find(|row| row.id == id) else {
            return Ok(false);
        };
        row.last_evaluated_at = Some(last_evaluated_at);
        Ok(true)
    }
}

#[async_trait::async_trait]
impl AlertRuleConfigurationInterface for FailingStore {
    async fn insert_alert_rule_configuration(
        &self,
        _new: AlertRuleConfigurationNew,
    ) -> StorageResult<AlertRuleConfiguration> {
        Err(report!(DatabaseError::Others))
    }

    async fn update_alert_rule_configuration(
        &self,
        _id: String,
        _update: AlertRuleConfigurationUpdate,
    ) -> StorageResult<AlertRuleConfiguration> {
        Err(report!(DatabaseError::Others))
    }

    async fn list_alert_rule_configurations(&self) -> StorageResult<AlertRuleConfigurationListing> {
        Err(report!(DatabaseError::Others))
    }

    async fn list_due_alert_rule_configurations(
        &self,
        _now: time::PrimitiveDateTime,
    ) -> StorageResult<AlertRuleConfigurationListing> {
        Err(report!(DatabaseError::Others))
    }

    async fn set_alert_rule_next_run_at(
        &self,
        _id: String,
        _next_run_at: time::PrimitiveDateTime,
    ) -> StorageResult<bool> {
        Err(report!(DatabaseError::Others))
    }

    async fn set_alert_rule_last_evaluated_at(
        &self,
        _id: String,
        _last_evaluated_at: time::PrimitiveDateTime,
    ) -> StorageResult<bool> {
        Err(report!(DatabaseError::Others))
    }
}

macro_rules! impl_unused_interfaces {
    ($store:ty) => {
        #[async_trait::async_trait]
        impl observability::db::alerts_info::AlertsInfoInterface for $store {
            async fn insert_alert_info(
                &self,
                _new: observability::domain_models::alerts_info::AlertsInfoNew,
            ) -> StorageResult<observability::domain_models::alerts_info::AlertsInfo> {
                Err(report!(DatabaseError::Others))
            }
        }

        #[async_trait::async_trait]
        impl observability::db::blacklist::BlacklistInterface for $store {
            async fn list_blacklist_entries(
                &self,
            ) -> StorageResult<Vec<observability::domain_models::blacklist::BlacklistEntry>> {
                Err(report!(DatabaseError::Others))
            }

            async fn upsert_blacklist_entry(
                &self,
                _new: observability::domain_models::blacklist::BlacklistEntryNew,
                _max_active_rules: i64,
            ) -> StorageResult<observability::domain_models::blacklist::BlacklistUpsertOutcome>
            {
                Err(report!(DatabaseError::Others))
            }

            async fn delete_blacklist_entry(
                &self,
                _new: observability::domain_models::blacklist::BlacklistEntryNew,
            ) -> StorageResult<observability::domain_models::blacklist::BlacklistEntry> {
                Err(report!(DatabaseError::Others))
            }
        }

        #[async_trait::async_trait]
        impl observability::db::dictionary::DictionaryInterface for $store {
            async fn list_dictionary_entries(
                &self,
            ) -> StorageResult<Vec<observability::domain_models::dictionary::DictionaryEntry>> {
                Err(report!(DatabaseError::Others))
            }

            async fn upsert_dictionary_entry(
                &self,
                _new: observability::domain_models::dictionary::DictionaryEntryNew,
            ) -> StorageResult<observability::domain_models::dictionary::DictionaryEntry> {
                Err(report!(DatabaseError::Others))
            }
        }

        #[async_trait::async_trait]
        impl observability::db::lifecycle_events::LifecycleEventsInterface for $store {
            async fn list_lifecycle_events(
                &self,
                _from: Option<time::PrimitiveDateTime>,
                _to: Option<time::PrimitiveDateTime>,
            ) -> StorageResult<Vec<observability::domain_models::lifecycle_events::LifecycleEvent>>
            {
                Err(report!(DatabaseError::Others))
            }

            async fn replace_lifecycle_events(
                &self,
                _batch: observability::domain_models::lifecycle_events::LifecycleEventsBatch,
            ) -> StorageResult<usize> {
                Err(report!(DatabaseError::Others))
            }
        }

        #[async_trait::async_trait]
        impl observability::db::metadata::AlertMetadataInterface for $store {
            async fn list_alert_metadata(
                &self,
            ) -> StorageResult<Vec<observability::domain_models::metadata::AlertMetadataEntry>>
            {
                Err(report!(DatabaseError::Others))
            }

            async fn patch_alert_metadata(
                &self,
                _patch: observability::domain_models::metadata::AlertMetadataPatch,
            ) -> StorageResult<observability::domain_models::metadata::AlertMetadataEntry> {
                Err(report!(DatabaseError::Others))
            }
        }

        #[async_trait::async_trait]
        impl observability::db::rule_toggles::RuleTogglesInterface for $store {
            async fn list_rule_toggles(
                &self,
            ) -> StorageResult<Vec<observability::domain_models::rule_toggles::RuleToggle>> {
                Err(report!(DatabaseError::Others))
            }

            async fn set_rule_toggle(
                &self,
                _new: observability::domain_models::rule_toggles::RuleToggleNew,
            ) -> StorageResult<observability::domain_models::rule_toggles::RuleToggle> {
                Err(report!(DatabaseError::Others))
            }
        }

        #[async_trait::async_trait]
        impl observability::db::thresholds::ThresholdsInterface for $store {
            async fn list_threshold_overrides(
                &self,
            ) -> StorageResult<Vec<observability::domain_models::thresholds::ThresholdOverride>>
            {
                Err(report!(DatabaseError::Others))
            }

            async fn upsert_threshold_override(
                &self,
                _new: observability::domain_models::thresholds::ThresholdOverrideNew,
                _max_active_rules: i64,
            ) -> StorageResult<observability::domain_models::thresholds::ThresholdUpsertOutcome>
            {
                Err(report!(DatabaseError::Others))
            }

            async fn delete_threshold_override(
                &self,
                _new: observability::domain_models::thresholds::ThresholdOverrideNew,
            ) -> StorageResult<observability::domain_models::thresholds::ThresholdOverride> {
                Err(report!(DatabaseError::Others))
            }
        }
    };
}

impl_unused_interfaces!(MemoryStore);
impl_unused_interfaces!(FailingStore);
impl StorageInterface for MemoryStore {}
impl StorageInterface for FailingStore {}

fn state_with_store(store: Arc<dyn StorageInterface>) -> AppState {
    let conf: observability::Settings = serde_json::from_value(json!({
        "auth": { "internal_api_key": API_KEY }
    }))
    .unwrap();
    AppState {
        router_transport: None,
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

fn query() -> Value {
    json!({
        "region": "us-east-1",
        "namespace": "AWS/EC2",
        "metric_name": "CPUUtilization",
        "dimensions": {"InstanceId": "i-053c2d51a2fc3535d"},
        "statistic": "Average",
        "period": 60
    })
}

fn thresholds(threshold: u32) -> Value {
    json!({
        "severities": {
            "sev1": {
                "threshold": threshold,
                "comparison_operator": "GreaterThanOrEqualToThreshold",
                "evaluation_periods": 1,
                "treat_missing_data": "breaching",
                "description": "SEV1 CPU utilization"
            }
        }
    })
}

fn create_payload(id: &str, name: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "classification": "kafka-alerts",
        "datasource": "cloudwatch",
        "query": query(),
        "thresholds": thresholds(80),
        "interval_seconds": 60
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
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

#[actix_web::test]
async fn create_list_update_uses_plain_api_and_id_ordering() {
    let state = state();
    let (_, zulu) = call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload("zulu-cpu", "Zulu CPU")),
    )
    .await;
    let (status, alpha) = call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload("alpha-cpu", "Alpha CPU")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(alpha["id"], "alpha-cpu");
    assert_eq!(alpha["is_enabled"], true);
    assert_eq!(alpha["last_evaluated_at"], Value::Null);
    assert_eq!(alpha["query"]["end_offset_seconds"], 180);
    assert!(alpha["query"].get("version").is_none());
    assert!(alpha["thresholds"].get("version").is_none());

    let (_, listed) = call(
        state.clone(),
        actix_web::http::Method::GET,
        COLLECTION_URI,
        Some(API_KEY),
        None,
    )
    .await;
    assert_eq!(listed["rule_configurations"][0]["id"], "alpha-cpu");
    assert_eq!(listed["rule_configurations"][1]["id"], "zulu-cpu");
    assert_eq!(listed["unreadable_ids"], json!([]));

    let (status, updated) = call(
        state.clone(),
        actix_web::http::Method::PUT,
        &format!("{COLLECTION_URI}/zulu-cpu"),
        Some(API_KEY),
        Some(json!({
            "classification": "updated-alerts",
            "thresholds": thresholds(85),
            "is_enabled": false
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["classification"], "updated-alerts");
    assert_eq!(
        updated["thresholds"]["severities"]["sev1"]["threshold"],
        85.0
    );
    assert!(updated["thresholds"].get("version").is_none());
    assert_eq!(updated["query"], zulu["query"]);
    assert_eq!(updated["next_run_at"], zulu["next_run_at"]);
    assert_eq!(updated["created_at"], zulu["created_at"]);
    assert_ne!(updated["last_updated_at"], zulu["last_updated_at"]);
}

#[actix_web::test]
async fn duplicate_id_and_name_return_400_and_unknown_update_returns_ir_09() {
    let state = state();
    call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload("duplicate", "Duplicate")),
    )
    .await;
    for payload in [
        create_payload("duplicate", "Other"),
        create_payload("other", "Duplicate"),
    ] {
        assert_eq!(
            call(
                state.clone(),
                actix_web::http::Method::POST,
                COLLECTION_URI,
                Some(API_KEY),
                Some(payload),
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }

    let (status, body) = call(
        state,
        actix_web::http::Method::PUT,
        &format!("{COLLECTION_URI}/unknown"),
        Some(API_KEY),
        Some(json!({"is_enabled": false})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.to_string().contains("IR_09"));
}

#[actix_web::test]
async fn validation_and_unknown_fields_return_400() {
    let mut invalid_interval = create_payload("invalid", "Invalid");
    invalid_interval["interval_seconds"] = json!(90);
    let mut ignored = create_payload("ignored", "Ignored");
    ignored["thresholds"]["severities"]["sev1"]["treat_missing_data"] = json!("ignore");
    let mut api_version = create_payload("versioned", "Versioned");
    api_version["query"]["version"] = json!("v1");

    for (method, uri, payload) in [
        (
            actix_web::http::Method::POST,
            COLLECTION_URI.to_owned(),
            invalid_interval,
        ),
        (actix_web::http::Method::POST, COLLECTION_URI.to_owned(), {
            let mut payload = create_payload("invalid-offset", "Invalid offset");
            payload["query"]["end_offset_seconds"] = json!(3601);
            payload
        }),
        (
            actix_web::http::Method::POST,
            COLLECTION_URI.to_owned(),
            ignored,
        ),
        (
            actix_web::http::Method::POST,
            COLLECTION_URI.to_owned(),
            api_version,
        ),
        (actix_web::http::Method::POST, COLLECTION_URI.to_owned(), {
            let mut payload = create_payload("extra", "Extra");
            payload["extra"] = json!(true);
            payload
        }),
        (
            actix_web::http::Method::PUT,
            format!("{COLLECTION_URI}/a"),
            json!({"is_enabled": false, "extra": true}),
        ),
        (
            actix_web::http::Method::PUT,
            format!("{COLLECTION_URI}/a"),
            json!({}),
        ),
    ] {
        assert_eq!(
            call(state(), method, &uri, Some(API_KEY), Some(payload))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}

#[actix_web::test]
async fn due_selection_and_evaluator_setters_are_isolated() {
    let store = Arc::new(MemoryStore::default());
    let state = state_with_store(store.clone());
    for (id, name) in [
        ("due", "Due"),
        ("future", "Future"),
        ("disabled", "Disabled"),
    ] {
        call(
            state.clone(),
            actix_web::http::Method::POST,
            COLLECTION_URI,
            Some(API_KEY),
            Some(create_payload(id, name)),
        )
        .await;
    }
    let future = datetime!(2026-09-15 09:49:00);
    assert!(store
        .set_alert_rule_next_run_at("future".to_owned(), future)
        .await
        .unwrap());
    call(
        state,
        actix_web::http::Method::PUT,
        &format!("{COLLECTION_URI}/disabled"),
        Some(API_KEY),
        Some(json!({"is_enabled": false})),
    )
    .await;

    let due = store
        .list_due_alert_rule_configurations(datetime!(2026-09-15 09:50:00))
        .await
        .unwrap();
    assert_eq!(due.rules.len(), 2);
    assert_eq!(due.rules[0].id, "due");
    assert_eq!(due.rules[1].id, "future");

    let before = store.rows.lock().unwrap()[0].clone();
    let evaluated = datetime!(2026-09-15 09:47:00);
    assert!(store
        .set_alert_rule_last_evaluated_at(before.id.clone(), evaluated)
        .await
        .unwrap());
    let after = store
        .rows
        .lock()
        .unwrap()
        .iter()
        .find(|row| row.id == before.id)
        .unwrap()
        .clone();
    assert_eq!(after.last_evaluated_at, Some(evaluated));
    assert_eq!(after.last_updated_at, before.last_updated_at);
    assert_eq!(after.next_run_at, before.next_run_at);
    assert_eq!(after.query, before.query);
    assert_eq!(after.thresholds, before.thresholds);
    assert!(!store
        .set_alert_rule_next_run_at("unknown".to_owned(), future)
        .await
        .unwrap());
    assert!(!store
        .set_alert_rule_last_evaluated_at("unknown".to_owned(), evaluated)
        .await
        .unwrap());
}

#[actix_web::test]
async fn every_endpoint_requires_the_correct_api_key() {
    for (method, uri, payload) in [
        (
            actix_web::http::Method::POST,
            COLLECTION_URI,
            Some(create_payload("auth", "Auth")),
        ),
        (actix_web::http::Method::GET, COLLECTION_URI, None),
        (
            actix_web::http::Method::PUT,
            "/alerts/alerts_manager/rule_configurations/a",
            Some(json!({"is_enabled": false})),
        ),
    ] {
        for key in [None, Some("wrong")] {
            assert_eq!(
                call(state(), method.clone(), uri, key, payload.clone())
                    .await
                    .0,
                StatusCode::UNAUTHORIZED
            );
        }
    }
}

#[actix_web::test]
async fn storage_failure_on_every_endpoint_returns_500() {
    let state = state_with_store(Arc::new(FailingStore));
    for (method, uri, payload) in [
        (
            actix_web::http::Method::POST,
            COLLECTION_URI,
            Some(create_payload("failure", "Failure")),
        ),
        (actix_web::http::Method::GET, COLLECTION_URI, None),
        (
            actix_web::http::Method::PUT,
            "/alerts/alerts_manager/rule_configurations/a",
            Some(json!({"is_enabled": false})),
        ),
    ] {
        assert_eq!(
            call(state.clone(), method, uri, Some(API_KEY), payload)
                .await
                .0,
            StatusCode::INTERNAL_SERVER_ERROR
        );
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

#[actix_web::test]
#[ignore = "requires OBSERVABILITY_TEST_DATABASE_URL and applied observability migrations"]
async fn postgres_repository() {
    use diesel_models::observability::{
        alert_manager::alert_rule_configurations::AlertRuleConfiguration as StorageRule,
        schema::alert_rule_configurations,
    };

    let database_url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .expect("OBSERVABILITY_TEST_DATABASE_URL is required");
    let database = postgres_database(&database_url);
    let unique = common_utils::generate_time_ordered_id("arc");
    let ids = [
        format!("{unique}-a"),
        format!("{unique}-z"),
        format!("{unique}-disabled"),
        format!("{unique}-bad-json"),
        format!("{unique}-bad-source"),
    ];
    let mut connection = PgConnection::establish(&database_url).unwrap();
    diesel::delete(alert_rule_configurations::table)
        .filter(alert_rule_configurations::id.eq_any(&ids))
        .execute(&mut connection)
        .unwrap();

    let store = Arc::new(observability::db::Store::new(&database).await.unwrap());
    let state = state_with_store(store.clone());
    for (id, name, enabled) in [
        (&ids[1], format!("{unique}-Zulu"), true),
        (&ids[0], format!("{unique}-Alpha"), true),
        (&ids[2], format!("{unique}-Disabled"), false),
    ] {
        let mut payload = create_payload(id, &name);
        payload["is_enabled"] = json!(enabled);
        assert_eq!(
            call(
                state.clone(),
                actix_web::http::Method::POST,
                COLLECTION_URI,
                Some(API_KEY),
                Some(payload),
            )
            .await
            .0,
            StatusCode::OK
        );
    }

    let stored: StorageRule = alert_rule_configurations::table
        .find(&ids[0])
        .first(&mut connection)
        .unwrap();
    assert_eq!(stored.query["version"], "v1");
    assert_eq!(stored.query["end_offset_seconds"], 180);
    assert_eq!(stored.thresholds["version"], "v1");
    let original = stored.clone();

    let (_, created) = call(
        state.clone(),
        actix_web::http::Method::GET,
        COLLECTION_URI,
        Some(API_KEY),
        None,
    )
    .await;
    let response = created["rule_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == ids[0])
        .unwrap();
    assert!(response["query"].get("version").is_none());
    assert!(response["thresholds"].get("version").is_none());

    let (status, updated) = call(
        state.clone(),
        actix_web::http::Method::PUT,
        &format!("{COLLECTION_URI}/{}", ids[0]),
        Some(API_KEY),
        Some(json!({"thresholds": thresholds(85), "is_enabled": false})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["query"], response["query"]);
    assert_eq!(updated["next_run_at"], response["next_run_at"]);
    assert_eq!(updated["created_at"], response["created_at"]);
    assert_ne!(updated["last_updated_at"], response["last_updated_at"]);

    let after_api: StorageRule = alert_rule_configurations::table
        .find(&ids[0])
        .first(&mut connection)
        .unwrap();
    assert_eq!(after_api.query, original.query);
    assert_eq!(after_api.name, original.name);
    assert_eq!(after_api.classification, original.classification);
    assert_eq!(after_api.datasource, original.datasource);
    assert_eq!(after_api.interval_seconds, original.interval_seconds);
    assert_eq!(after_api.created_at, original.created_at);

    let next_run = datetime!(2026-09-16 09:45:00);
    assert!(store
        .set_alert_rule_next_run_at(ids[0].clone(), next_run)
        .await
        .unwrap());
    let after_next: StorageRule = alert_rule_configurations::table
        .find(&ids[0])
        .first(&mut connection)
        .unwrap();
    assert_eq!(after_next.next_run_at, next_run);
    assert_eq!(after_next.id, after_api.id);
    assert_eq!(after_next.name, after_api.name);
    assert_eq!(after_next.classification, after_api.classification);
    assert_eq!(after_next.datasource, after_api.datasource);
    assert_eq!(after_next.query, after_api.query);
    assert_eq!(after_next.thresholds, after_api.thresholds);
    assert_eq!(after_next.interval_seconds, after_api.interval_seconds);
    assert_eq!(after_next.is_enabled, after_api.is_enabled);
    assert_eq!(after_next.last_evaluated_at, after_api.last_evaluated_at);
    assert_eq!(after_next.created_at, after_api.created_at);
    assert_eq!(after_next.last_updated_at, after_api.last_updated_at);

    let evaluated = datetime!(2026-09-15 09:47:00);
    assert!(store
        .set_alert_rule_last_evaluated_at(ids[0].clone(), evaluated)
        .await
        .unwrap());
    let after_evaluated: StorageRule = alert_rule_configurations::table
        .find(&ids[0])
        .first(&mut connection)
        .unwrap();
    assert_eq!(after_evaluated.id, after_next.id);
    assert_eq!(after_evaluated.name, after_next.name);
    assert_eq!(after_evaluated.classification, after_next.classification);
    assert_eq!(after_evaluated.datasource, after_next.datasource);
    assert_eq!(after_evaluated.query, after_next.query);
    assert_eq!(after_evaluated.thresholds, after_next.thresholds);
    assert_eq!(
        after_evaluated.interval_seconds,
        after_next.interval_seconds
    );
    assert_eq!(after_evaluated.is_enabled, after_next.is_enabled);
    assert_eq!(after_evaluated.next_run_at, after_next.next_run_at);
    assert_eq!(after_evaluated.last_evaluated_at, Some(evaluated));
    assert_eq!(after_evaluated.created_at, after_next.created_at);
    assert_eq!(after_evaluated.last_updated_at, after_next.last_updated_at);
    assert!(!store
        .set_alert_rule_next_run_at(format!("{unique}-unknown"), next_run)
        .await
        .unwrap());
    assert!(!store
        .set_alert_rule_last_evaluated_at(format!("{unique}-unknown"), evaluated)
        .await
        .unwrap());

    call(
        state,
        actix_web::http::Method::PUT,
        &format!("{COLLECTION_URI}/{}", ids[0]),
        Some(API_KEY),
        Some(json!({"is_enabled": true})),
    )
    .await;
    let due_time = datetime!(2026-09-15 09:40:00);
    store
        .set_alert_rule_next_run_at(ids[1].clone(), due_time)
        .await
        .unwrap();
    store
        .set_alert_rule_next_run_at(ids[0].clone(), datetime!(2026-09-15 09:45:00))
        .await
        .unwrap();
    store
        .set_alert_rule_next_run_at(ids[2].clone(), due_time)
        .await
        .unwrap();
    let due = store
        .list_due_alert_rule_configurations(datetime!(2026-09-15 09:50:00))
        .await
        .unwrap();
    let owned_due: Vec<_> = due
        .rules
        .iter()
        .filter(|rule| ids.contains(&rule.id))
        .map(|rule| rule.id.as_str())
        .collect();
    assert_eq!(owned_due, vec![ids[1].as_str(), ids[0].as_str()]);

    let valid_query = json!({
        "version": "v1",
        "region": "us-east-1",
        "namespace": "AWS/EC2",
        "metric_name": "CPUUtilization",
        "dimensions": {},
        "statistic": "Average",
        "period": 60
    });
    let valid_thresholds = json!({
        "version": "v1",
        "severities": {
            "sev1": {
                "threshold": 80.0,
                "comparison_operator": "GreaterThanOrEqualToThreshold",
                "evaluation_periods": 1,
                "treat_missing_data": "missing",
                "description": "bad row fixture"
            }
        }
    });
    diesel::insert_into(alert_rule_configurations::table)
        .values((
            alert_rule_configurations::id.eq(&ids[3]),
            alert_rule_configurations::name.eq(format!("{unique}-BadJson")),
            alert_rule_configurations::classification.eq("kafka-alerts"),
            alert_rule_configurations::datasource.eq("cloudwatch"),
            alert_rule_configurations::query.eq(json!({"version": "unknown"})),
            alert_rule_configurations::thresholds.eq(&valid_thresholds),
            alert_rule_configurations::interval_seconds.eq(60),
            alert_rule_configurations::is_enabled.eq(true),
        ))
        .execute(&mut connection)
        .unwrap();
    diesel::insert_into(alert_rule_configurations::table)
        .values((
            alert_rule_configurations::id.eq(&ids[4]),
            alert_rule_configurations::name.eq(format!("{unique}-BadSource")),
            alert_rule_configurations::classification.eq("kafka-alerts"),
            alert_rule_configurations::datasource.eq("unknown"),
            alert_rule_configurations::query.eq(&valid_query),
            alert_rule_configurations::thresholds.eq(&valid_thresholds),
            alert_rule_configurations::interval_seconds.eq(60),
            alert_rule_configurations::is_enabled.eq(true),
        ))
        .execute(&mut connection)
        .unwrap();
    let listing = store.list_alert_rule_configurations().await.unwrap();
    assert!(listing.rules.iter().any(|rule| rule.id == ids[1]));
    assert!(listing.unreadable_ids.contains(&ids[3]));
    assert!(listing.unreadable_ids.contains(&ids[4]));

    diesel::delete(alert_rule_configurations::table)
        .filter(alert_rule_configurations::id.eq_any(&ids))
        .execute(&mut connection)
        .unwrap();
}
