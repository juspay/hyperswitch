use euclid::frontend::ast::Program;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreeDsDecisionRuleRecord {
    pub name: String,
    pub description: Option<String>,
    pub program: Program<common_types::three_ds_decision_rule_engine::ThreeDSDecisionRule>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreeDsDecisionRuleResponse {
    pub id: common_utils::id_type::ThreeDSDecisionRuleId,
    pub name: String,
    pub description: Option<String>,
    pub program: Program<common_types::three_ds_decision_rule_engine::ThreeDSDecisionRule>,
    pub active: bool,
}

impl common_utils::events::ApiEventMetric for ThreeDsDecisionRuleRecord {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::ThreeDsDecisionRule)
    }
}

impl common_utils::events::ApiEventMetric for ThreeDsDecisionRuleResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::ThreeDsDecisionRule)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreeDsDecisionRuleUpdateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub program: Option<Program<common_types::three_ds_decision_rule_engine::ThreeDSDecisionRule>>,
}

impl common_utils::events::ApiEventMetric for ThreeDsDecisionRuleUpdateRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::ThreeDsDecisionRule)
    }
}
