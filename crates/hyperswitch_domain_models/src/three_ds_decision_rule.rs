use common_utils::{
    self,
    errors::{CustomResult, ValidationError},
    id_type::{self, GenerateId},
    types::keymanager,
};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThreeDSDecisionRule {
    pub id: id_type::ThreeDSDecisionRuleId,
    pub rule: common_types::three_ds_decision_rule_engine::ThreeDSDecisionRuleRecord,
    pub name: String,
    pub description: Option<String>,
    pub active: bool,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
}

impl ThreeDSDecisionRule {
    pub fn new(
        rule: common_types::three_ds_decision_rule_engine::ThreeDSDecisionRuleRecord,
        name: String,
        description: Option<String>,
    ) -> Self {
        let id = id_type::ThreeDSDecisionRuleId::generate();
        let now = common_utils::date_time::now();
        Self {
            id,
            rule,
            name,
            description,
            active: true,
            created_at: now,
            modified_at: now,
        }
    }
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

#[async_trait::async_trait]
impl super::behaviour::Conversion for ThreeDSDecisionRule {
    type DstType = diesel_models::three_ds_decision_rule::ThreeDSDecisionRule;
    type NewDstType = diesel_models::three_ds_decision_rule::ThreeDSDecisionRuleNew;

    async fn convert(self) -> CustomResult<Self::DstType, ValidationError> {
        Ok(diesel_models::three_ds_decision_rule::ThreeDSDecisionRule {
            id: self.id,
            rule: self.rule,
            name: self.name,
            description: self.description,
            active: self.active,
            created_at: self.created_at,
            modified_at: self.modified_at,
        })
    }

    async fn convert_back(
        _state: &keymanager::KeyManagerState,
        item: Self::DstType,
        _key: &Secret<Vec<u8>>,
        _key_manager_identifier: keymanager::Identifier,
    ) -> CustomResult<Self, ValidationError> {
        Ok(Self {
            id: item.id,
            rule: item.rule,
            name: item.name,
            description: item.description,
            active: item.active,
            created_at: item.created_at,
            modified_at: item.modified_at,
        })
    }

    async fn construct_new(self) -> CustomResult<Self::NewDstType, ValidationError> {
        Ok(
            diesel_models::three_ds_decision_rule::ThreeDSDecisionRuleNew {
                id: self.id,
                rule: self.rule,
                name: self.name,
                description: self.description,
                active: self.active,
                created_at: self.created_at,
                modified_at: self.modified_at,
            },
        )
    }
}

impl From<ThreeDSDecisionRuleUpdate>
    for diesel_models::three_ds_decision_rule::ThreeDSDecisionRuleUpdate
{
    fn from(value: ThreeDSDecisionRuleUpdate) -> Self {
        match value {
            ThreeDSDecisionRuleUpdate::Update {
                rule,
                name,
                description,
            } => Self::Update {
                rule,
                name,
                description,
            },
            ThreeDSDecisionRuleUpdate::Delete => Self::Delete,
        }
    }
}
