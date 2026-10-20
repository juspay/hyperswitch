#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use actix_web::{http::StatusCode, test, App};
use diesel::{Connection, ExpressionMethods, PgConnection, RunQueryDsl};
use diesel_models::{
    errors::DatabaseError,
    observability::{
        alert_manager::alert_rule_configurations::AlertRuleConfiguration as StorageRule,
        schema::alert_rule_configurations,
    },
    StorageResult,
};
use error_stack::report;
use observability::{
    auth::X_INTERNAL_API_KEY,
    db::{alert_rule_configurations::AlertRuleConfigurationInterface, StorageInterface},
    domain::notifier::Registry,
    domain_models::alert_rule_configurations::{
        AlertRuleConfigurationNew, AlertRuleConfigurationUpdate,
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
    rows: Mutex<Vec<StorageRule>>,
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
    ) -> StorageResult<StorageRule> {
        let mut rows = self.rows.lock().unwrap();
        if rows.iter().any(|row| row.name == new.name) {
            return Err(report!(DatabaseError::UniqueViolation));
        }
        let row = StorageRule {
            id: new.id,
            name: new.name,
            is_enabled: new.is_enabled,
            config: serde_json::to_value(new.config).unwrap(),
            next_run_at: initial_time(),
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
    ) -> StorageResult<StorageRule> {
        let mut rows = self.rows.lock().unwrap();
        let row = rows
            .iter_mut()
            .find(|row| row.id == id)
            .ok_or_else(|| report!(DatabaseError::NotFound))?;
        match update {
            AlertRuleConfigurationUpdate::Update { config, is_enabled } => {
                if let Some(config) = config {
                    row.config = serde_json::to_value(config).unwrap();
                }
                if let Some(is_enabled) = is_enabled {
                    row.is_enabled = is_enabled;
                }
            }
        }
        row.last_updated_at = updated_time();
        Ok(row.clone())
    }

    async fn list_alert_rule_configurations(&self) -> StorageResult<Vec<StorageRule>> {
        let mut rows = self.rows.lock().unwrap().clone();
        rows.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(rows)
    }
}

#[async_trait::async_trait]
impl AlertRuleConfigurationInterface for FailingStore {
    async fn insert_alert_rule_configuration(
        &self,
        _new: AlertRuleConfigurationNew,
    ) -> StorageResult<StorageRule> {
        Err(report!(DatabaseError::Others))
    }

    async fn update_alert_rule_configuration(
        &self,
        _id: String,
        _update: AlertRuleConfigurationUpdate,
    ) -> StorageResult<StorageRule> {
        Err(report!(DatabaseError::Others))
    }

    async fn list_alert_rule_configurations(&self) -> StorageResult<Vec<StorageRule>> {
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

fn config(threshold: u32) -> Value {
    json!({
        "version": "v1",
        "query": {
            "version": "v1",
            "data_source": "cloudwatch",
            "region": "us-east-1",
            "namespace": "AWS/EC2",
            "metric_name": "CPUUtilization",
            "dimensions": {"InstanceId": "i-053c2d51a2fc3535d"},
            "statistic": "Average",
            "period": 60
        },
        "frequency": 60,
        "severities": {
            "sev1": {
                "operator": "gte",
                "threshold": threshold,
                "description": "SEV1 CPU utilization"
            }
        }
    })
}

fn create_payload(name: &str) -> Value {
    json!({"name": name, "config": config(80)})
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
async fn create_list_update_list_is_ordered_and_preserves_schedule() {
    let state = state();
    let (_, zulu) = call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload("Zulu CPU")),
    )
    .await;
    let (status, alpha) = call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload("Alpha CPU")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(alpha["id"].as_str().unwrap().starts_with("rule_"));
    assert_eq!(alpha["is_enabled"], true);

    let (_, listed) = call(
        state.clone(),
        actix_web::http::Method::GET,
        COLLECTION_URI,
        Some(API_KEY),
        None,
    )
    .await;
    assert_eq!(listed["rule_configurations"][0]["name"], "Alpha CPU");
    assert_eq!(listed["rule_configurations"][1]["name"], "Zulu CPU");
    assert_eq!(listed["unreadable_ids"], json!([]));

    let uri = format!("{COLLECTION_URI}/{}", zulu["id"].as_str().unwrap());
    let (status, updated) = call(
        state.clone(),
        actix_web::http::Method::PUT,
        &uri,
        Some(API_KEY),
        Some(json!({"is_enabled": false, "config": config(85)})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["is_enabled"], false);
    assert_eq!(updated["config"]["severities"]["sev1"]["threshold"], 85.0);
    assert_eq!(updated["next_run_at"], zulu["next_run_at"]);
    assert_eq!(updated["created_at"], zulu["created_at"]);
    assert_ne!(updated["last_updated_at"], zulu["last_updated_at"]);

    let (_, listed) = call(
        state,
        actix_web::http::Method::GET,
        COLLECTION_URI,
        Some(API_KEY),
        None,
    )
    .await;
    assert_eq!(listed["rule_configurations"][1], updated);
}

#[actix_web::test]
async fn duplicate_name_and_unknown_id_return_400() {
    let state = state();
    call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload("Duplicate")),
    )
    .await;
    let (status, _) = call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload("Duplicate")),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = call(
        state,
        actix_web::http::Method::PUT,
        &format!("{COLLECTION_URI}/rule_unknown"),
        Some(API_KEY),
        Some(json!({"is_enabled": false})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn invalid_config_and_unknown_fields_return_400() {
    let mut invalid = config(80);
    invalid["frequency"] = json!(0);
    let mut sev4 = config(80);
    sev4["severities"]["sev4"] = sev4["severities"]["sev1"].clone();

    for (method, uri, payload) in [
        (
            actix_web::http::Method::POST,
            COLLECTION_URI.to_owned(),
            json!({"name": "Invalid", "config": invalid}),
        ),
        (
            actix_web::http::Method::POST,
            COLLECTION_URI.to_owned(),
            json!({"name": "Unknown", "config": config(80), "extra": true}),
        ),
        (
            actix_web::http::Method::POST,
            COLLECTION_URI.to_owned(),
            json!({"name": "Severity", "config": sev4}),
        ),
        (
            actix_web::http::Method::PUT,
            format!("{COLLECTION_URI}/rule_a"),
            json!({"is_enabled": false, "extra": true}),
        ),
        (
            actix_web::http::Method::PUT,
            format!("{COLLECTION_URI}/rule_a"),
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
async fn every_endpoint_requires_the_correct_api_key() {
    for (method, uri, payload) in [
        (
            actix_web::http::Method::POST,
            COLLECTION_URI,
            Some(create_payload("Auth")),
        ),
        (actix_web::http::Method::GET, COLLECTION_URI, None),
        (
            actix_web::http::Method::PUT,
            "/alerts/alerts_manager/rule_configurations/rule_a",
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
            Some(create_payload("Failure")),
        ),
        (actix_web::http::Method::GET, COLLECTION_URI, None),
        (
            actix_web::http::Method::PUT,
            "/alerts/alerts_manager/rule_configurations/rule_a",
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

#[actix_web::test]
async fn malformed_rows_are_isolated() {
    let store = Arc::new(MemoryStore::default());
    store.rows.lock().unwrap().extend([
        StorageRule {
            id: "rule_bad".to_owned(),
            name: "Bad".to_owned(),
            is_enabled: true,
            config: json!({"version": "unknown"}),
            next_run_at: initial_time(),
            created_at: initial_time(),
            last_updated_at: initial_time(),
        },
        StorageRule {
            id: "rule_good".to_owned(),
            name: "Good".to_owned(),
            is_enabled: true,
            config: config(80),
            next_run_at: initial_time(),
            created_at: initial_time(),
            last_updated_at: initial_time(),
        },
    ]);
    let (status, listed) = call(
        state_with_store(store),
        actix_web::http::Method::GET,
        COLLECTION_URI,
        Some(API_KEY),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(listed["rule_configurations"].as_array().unwrap().len(), 1);
    assert_eq!(listed["rule_configurations"][0]["id"], "rule_good");
    assert_eq!(listed["unreadable_ids"], json!(["rule_bad"]));
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
    let database_url = std::env::var("OBSERVABILITY_TEST_DATABASE_URL")
        .expect("OBSERVABILITY_TEST_DATABASE_URL is required");
    let database = postgres_database(&database_url);
    let unique = common_utils::generate_time_ordered_id("alert_rule_configuration_test");
    let names = [format!("{unique}-Zulu"), format!("{unique}-Alpha")];
    let mut connection = PgConnection::establish(&database_url).unwrap();
    diesel::delete(alert_rule_configurations::table)
        .filter(alert_rule_configurations::name.eq_any(&names))
        .execute(&mut connection)
        .unwrap();

    let state = state_with_store(Arc::new(
        observability::db::Store::new(&database).await.unwrap(),
    ));
    let (_, zulu) = call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload(&names[0])),
    )
    .await;
    call(
        state.clone(),
        actix_web::http::Method::POST,
        COLLECTION_URI,
        Some(API_KEY),
        Some(create_payload(&names[1])),
    )
    .await;
    let uri = format!("{COLLECTION_URI}/{}", zulu["id"].as_str().unwrap());
    let (status, updated) = call(
        state.clone(),
        actix_web::http::Method::PUT,
        &uri,
        Some(API_KEY),
        Some(json!({"is_enabled": false, "config": config(85)})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["next_run_at"], zulu["next_run_at"]);
    assert_eq!(updated["created_at"], zulu["created_at"]);

    let (_, listed) = call(
        state,
        actix_web::http::Method::GET,
        COLLECTION_URI,
        Some(API_KEY),
        None,
    )
    .await;
    let owned: Vec<_> = listed["rule_configurations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| names.iter().any(|name| row["name"] == *name))
        .collect();
    assert_eq!(owned.len(), 2);
    assert_eq!(owned[0]["name"], names[1]);
    assert_eq!(owned[1]["name"], names[0]);
    assert_eq!(owned[1]["is_enabled"], false);
    assert_eq!(owned[1]["config"]["severities"]["sev1"]["threshold"], 85.0);

    diesel::delete(alert_rule_configurations::table)
        .filter(alert_rule_configurations::name.eq_any(&names))
        .execute(&mut connection)
        .unwrap();
}
