use common_enums::AlertRuleStatus;
use common_types::alert_rules::AlertRuleConfig;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_rules;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_rules)]
pub struct AlertRuleNew {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
    pub status: AlertRuleStatus,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = alert_rules, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub config: AlertRuleConfig,
    pub status: AlertRuleStatus,
    pub next_run_at: PrimitiveDateTime,
    pub last_evaluated_at: Option<PrimitiveDateTime>,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum AlertRuleUpdate {
    Update {
        name: Option<String>,
        config: Option<AlertRuleConfig>,
        status: Option<AlertRuleStatus>,
    },
    NextRun {
        next_run_at: PrimitiveDateTime,
    },
    Evaluated {
        last_evaluated_at: PrimitiveDateTime,
    },
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alert_rules)]
pub struct AlertRuleUpdateInternal {
    name: Option<String>,
    config: Option<AlertRuleConfig>,
    status: Option<AlertRuleStatus>,
    next_run_at: Option<PrimitiveDateTime>,
    last_evaluated_at: Option<PrimitiveDateTime>,
    last_updated_at: Option<PrimitiveDateTime>,
}

impl From<AlertRuleUpdate> for AlertRuleUpdateInternal {
    fn from(update: AlertRuleUpdate) -> Self {
        match update {
            AlertRuleUpdate::Update {
                name,
                config,
                status,
            } => Self {
                name,
                config,
                status,
                next_run_at: None,
                last_evaluated_at: None,
                last_updated_at: Some(common_utils::date_time::now()),
            },
            AlertRuleUpdate::NextRun { next_run_at } => Self {
                name: None,
                config: None,
                status: None,
                next_run_at: Some(next_run_at),
                last_evaluated_at: None,
                last_updated_at: None,
            },
            AlertRuleUpdate::Evaluated { last_evaluated_at } => Self {
                name: None,
                config: None,
                status: None,
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
        let update = AlertRuleUpdateInternal::from(AlertRuleUpdate::Update {
            name: Some("name".to_owned()),
            config: None,
            status: None,
        });

        assert_eq!(update.name.as_deref(), Some("name"));
        assert!(update.config.is_none());
        assert!(update.status.is_none());
        assert!(update.next_run_at.is_none());
        assert!(update.last_evaluated_at.is_none());
        assert!(update.last_updated_at.is_some());
    }

    #[test]
    fn evaluator_updates_only_their_column() {
        let timestamp = datetime!(2026-09-15 09:45:00);
        let next_run = AlertRuleUpdateInternal::from(AlertRuleUpdate::NextRun {
            next_run_at: timestamp,
        });
        assert_eq!(next_run.next_run_at, Some(timestamp));
        assert!(next_run.last_evaluated_at.is_none());
        assert!(next_run.last_updated_at.is_none());
        assert!(next_run.name.is_none());
        assert!(next_run.config.is_none());
        assert!(next_run.status.is_none());

        let evaluated = AlertRuleUpdateInternal::from(AlertRuleUpdate::Evaluated {
            last_evaluated_at: timestamp,
        });
        assert!(evaluated.next_run_at.is_none());
        assert_eq!(evaluated.last_evaluated_at, Some(timestamp));
        assert!(evaluated.last_updated_at.is_none());
        assert!(evaluated.name.is_none());
        assert!(evaluated.config.is_none());
        assert!(evaluated.status.is_none());
    }
}
