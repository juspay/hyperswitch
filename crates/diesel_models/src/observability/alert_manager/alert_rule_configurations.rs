use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_rule_configurations;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_rule_configurations)]
pub struct AlertRuleConfigurationNew {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub datasource: String,
    pub query: serde_json::Value,
    pub thresholds: serde_json::Value,
    pub interval_seconds: i32,
    pub is_enabled: bool,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = alert_rule_configurations, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertRuleConfiguration {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub datasource: String,
    pub query: serde_json::Value,
    pub thresholds: serde_json::Value,
    pub interval_seconds: i32,
    pub is_enabled: bool,
    pub next_run_at: PrimitiveDateTime,
    pub last_evaluated_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum AlertRuleConfigurationUpdate {
    Update {
        name: Option<String>,
        classification: Option<String>,
        query: Option<serde_json::Value>,
        thresholds: Option<serde_json::Value>,
        interval_seconds: Option<i32>,
        is_enabled: Option<bool>,
    },
    NextRun {
        next_run_at: PrimitiveDateTime,
    },
    Evaluated {
        last_evaluated_at: PrimitiveDateTime,
    },
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alert_rule_configurations)]
pub struct AlertRuleConfigurationUpdateInternal {
    name: Option<String>,
    classification: Option<String>,
    query: Option<serde_json::Value>,
    thresholds: Option<serde_json::Value>,
    interval_seconds: Option<i32>,
    is_enabled: Option<bool>,
    next_run_at: Option<PrimitiveDateTime>,
    last_evaluated_at: Option<PrimitiveDateTime>,
    last_updated_at: Option<PrimitiveDateTime>,
}

impl From<AlertRuleConfigurationUpdate> for AlertRuleConfigurationUpdateInternal {
    fn from(update: AlertRuleConfigurationUpdate) -> Self {
        match update {
            AlertRuleConfigurationUpdate::Update {
                name,
                classification,
                query,
                thresholds,
                interval_seconds,
                is_enabled,
            } => Self {
                name,
                classification,
                query,
                thresholds,
                interval_seconds,
                is_enabled,
                next_run_at: None,
                last_evaluated_at: None,
                last_updated_at: Some(common_utils::date_time::now()),
            },
            AlertRuleConfigurationUpdate::NextRun { next_run_at } => Self {
                name: None,
                classification: None,
                query: None,
                thresholds: None,
                interval_seconds: None,
                is_enabled: None,
                next_run_at: Some(next_run_at),
                last_evaluated_at: None,
                last_updated_at: None,
            },
            AlertRuleConfigurationUpdate::Evaluated { last_evaluated_at } => Self {
                name: None,
                classification: None,
                query: None,
                thresholds: None,
                interval_seconds: None,
                is_enabled: None,
                next_run_at: None,
                last_evaluated_at: Some(last_evaluated_at),
                last_updated_at: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn api_update_stamps_only_api_columns_and_last_updated_at() {
        let update =
            AlertRuleConfigurationUpdateInternal::from(AlertRuleConfigurationUpdate::Update {
                name: Some("name".to_owned()),
                classification: None,
                query: None,
                thresholds: None,
                interval_seconds: None,
                is_enabled: None,
            });

        assert_eq!(update.name.as_deref(), Some("name"));
        assert!(update.classification.is_none());
        assert!(update.query.is_none());
        assert!(update.thresholds.is_none());
        assert!(update.interval_seconds.is_none());
        assert!(update.is_enabled.is_none());
        assert!(update.next_run_at.is_none());
        assert!(update.last_evaluated_at.is_none());
        assert!(update.last_updated_at.is_some());
    }

    #[test]
    fn evaluator_updates_only_their_column() {
        let timestamp = datetime!(2026-09-15 09:45:00);
        let next_run =
            AlertRuleConfigurationUpdateInternal::from(AlertRuleConfigurationUpdate::NextRun {
                next_run_at: timestamp,
            });
        assert_eq!(next_run.next_run_at, Some(timestamp));
        assert!(next_run.last_evaluated_at.is_none());
        assert!(next_run.last_updated_at.is_none());
        assert!(next_run.name.is_none());
        assert!(next_run.classification.is_none());
        assert!(next_run.query.is_none());
        assert!(next_run.thresholds.is_none());
        assert!(next_run.interval_seconds.is_none());
        assert!(next_run.is_enabled.is_none());

        let evaluated =
            AlertRuleConfigurationUpdateInternal::from(AlertRuleConfigurationUpdate::Evaluated {
                last_evaluated_at: timestamp,
            });
        assert!(evaluated.next_run_at.is_none());
        assert_eq!(evaluated.last_evaluated_at, Some(timestamp));
        assert!(evaluated.last_updated_at.is_none());
        assert!(evaluated.name.is_none());
        assert!(evaluated.classification.is_none());
        assert!(evaluated.query.is_none());
        assert!(evaluated.thresholds.is_none());
        assert!(evaluated.interval_seconds.is_none());
        assert!(evaluated.is_enabled.is_none());
    }
}
