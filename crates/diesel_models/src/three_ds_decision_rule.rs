use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::three_ds_decision_rule;

#[derive(
    Clone, Debug, Identifiable, Queryable, Selectable, serde::Serialize, serde::Deserialize,
)]
#[diesel(table_name = three_ds_decision_rule)]
pub struct ThreeDSDecisionRule {
    pub id: common_utils::id_type::ThreeDSDecisionRuleId,
    pub rule: common_types::three_ds_decision_rule_engine::ThreeDSDecisionRuleRecord,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
}

#[derive(
    Clone,
    Debug,
    Insertable,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = three_ds_decision_rule)]
pub struct ThreeDSDecisionRuleNew {
    pub id: common_utils::id_type::ThreeDSDecisionRuleId,
    pub rule: common_types::three_ds_decision_rule_engine::ThreeDSDecisionRuleRecord,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = three_ds_decision_rule)]
pub struct ThreeDSDecisionRuleUpdateInternal {
    pub rule: Option<common_types::three_ds_decision_rule_engine::ThreeDSDecisionRuleRecord>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    pub modified_at: PrimitiveDateTime,
}

#[derive(Debug, Clone)]
pub enum ThreeDSDecisionRuleUpdate {
    Update {
        rule: Option<common_types::three_ds_decision_rule_engine::ThreeDSDecisionRuleRecord>,
        name: Option<String>,
        description: Option<String>,
    },
    Delete,
}

impl From<ThreeDSDecisionRuleUpdate> for ThreeDSDecisionRuleUpdateInternal {
    fn from(value: ThreeDSDecisionRuleUpdate) -> Self {
        match value {
            ThreeDSDecisionRuleUpdate::Update {
                rule,
                name,
                description,
            } => Self {
                rule,
                name,
                description,
                modified_at: common_utils::date_time::now(),
                active: None,
            },
            ThreeDSDecisionRuleUpdate::Delete => Self {
                rule: None,
                name: None,
                description: None,
                modified_at: common_utils::date_time::now(),
                active: Some(false),
            },
        }
    }
}
