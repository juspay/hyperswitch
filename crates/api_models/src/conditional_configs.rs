use common_utils::events;
use euclid::frontend::ast::Program;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionManagerRecord {
    pub name: String,
    pub program: Program<common_types::payments::ConditionalConfigs>,
    pub created_at: i64,
    pub modified_at: i64,
}
impl events::ApiEventMetric for DecisionManagerRecord {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalConfigReq {
    pub name: Option<String>,
    pub algorithm: Option<Program<common_types::payments::ConditionalConfigs>>,
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionManagerRequest {
    pub name: Option<String>,
    pub program: Option<Program<common_types::payments::ConditionalConfigs>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DecisionManager {
    DecisionManagerv0(ConditionalConfigReq),
    DecisionManagerv1(DecisionManagerRequest),
}

impl events::ApiEventMetric for DecisionManager {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}

pub type DecisionManagerResponse = DecisionManagerRecord;

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionManagerRequest {
    pub name: String,
    pub program: Program<common_types::payments::ConditionalConfigs>,
}

#[cfg(feature = "v2")]
impl events::ApiEventMetric for DecisionManagerRequest {
    fn get_api_event_type(&self) -> Option<events::ApiEventsType> {
        Some(events::ApiEventsType::Routing)
    }
}
