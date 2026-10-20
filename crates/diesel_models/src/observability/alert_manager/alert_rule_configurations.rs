use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::observability::schema::alert_rule_configurations;

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = alert_rule_configurations)]
pub struct AlertRuleConfigurationNew {
    pub id: String,
    pub name: String,
    pub is_enabled: bool,
    pub config: serde_json::Value,
}

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = alert_rule_configurations, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct AlertRuleConfiguration {
    pub id: String,
    pub name: String,
    pub is_enabled: bool,
    pub config: serde_json::Value,
    pub next_run_at: PrimitiveDateTime,
    pub created_at: PrimitiveDateTime,
    pub last_updated_at: PrimitiveDateTime,
}

#[derive(Debug)]
pub enum AlertRuleConfigurationUpdate {
    Update {
        config: Option<serde_json::Value>,
        is_enabled: Option<bool>,
    },
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = alert_rule_configurations)]
pub struct AlertRuleConfigurationUpdateInternal {
    config: Option<serde_json::Value>,
    is_enabled: Option<bool>,
    last_updated_at: PrimitiveDateTime,
}

impl From<AlertRuleConfigurationUpdate> for AlertRuleConfigurationUpdateInternal {
    fn from(update: AlertRuleConfigurationUpdate) -> Self {
        match update {
            AlertRuleConfigurationUpdate::Update { config, is_enabled } => Self {
                config,
                is_enabled,
                last_updated_at: common_utils::date_time::now(),
            },
        }
    }
}
