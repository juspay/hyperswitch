#[cfg(feature = "backwards_compatibility")]
pub use api_models::routing::RoutableChoiceKind;
pub use api_models::{
    enums as api_enums,
    routing::{
        ConnectorVolumeSplit, DetailedConnectorChoice, RoutableConnectorChoice, RoutingAlgorithm,
        RoutingAlgorithmKind, RoutingAlgorithmRef, RoutingConfigRequest, RoutingDictionary,
        RoutingDictionaryRecord, StraightThroughAlgorithm,
    },
};

use super::types::api as api_oss;

pub struct SessionRoutingChoice {
    pub connector: api_oss::ConnectorData,
    #[cfg(not(feature = "connector_choice_mca_id"))]
    pub sub_label: Option<String>,
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
