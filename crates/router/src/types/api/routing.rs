pub use api_models::{
    enums as api_enums,
    routing::{
        ConnectorVolumeSplit, RoutableChoiceKind, RoutableConnectorChoice, RoutingAlgorithmKind,
        RoutingAlgorithmRef, RoutingConfigRequest, RoutingDictionary, RoutingDictionaryRecord,
        StaticRoutingAlgorithm, StraightThroughAlgorithm,
    },
};

use super::types::api as api_oss;

pub struct SessionRoutingChoice {
    pub connector: api_oss::ConnectorData,
    pub payment_method_type: api_enums::PaymentMethodType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorVolumeSplitV0 {
    pub connector: RoutableConnectorChoice,
    pub split: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum RoutingAlgorithmV0 {
    Single(Box<RoutableConnectorChoice>),
    Priority(Vec<RoutableConnectorChoice>),
    VolumeSplit(Vec<ConnectorVolumeSplitV0>),
    Custom { timestamp: i64 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrmRoutingAlgorithm {
    pub data: String,
    #[serde(rename = "type")]
    pub algorithm_type: String,
}
