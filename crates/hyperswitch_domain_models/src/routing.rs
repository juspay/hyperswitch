use std::collections::HashMap;

use api_models::{enums as api_enums, routing};
use common_utils::id_type;

#[cfg(feature = "v1")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingData {
    pub routed_through: Option<String>,

    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    pub routing_info: PaymentRoutingInfo,
    pub algorithm: Option<routing::StraightThroughAlgorithm>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingData {
    // TODO: change this to RoutableConnectors enum
    pub routed_through: Option<String>,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,

    pub pre_routing_connector_choice: Option<PreRoutingConnectorChoice>,

    pub algorithm_requested: Option<id_type::RoutingId>,
}

/// The payment context a persisted pre-routing decision was computed under. Confirm
/// refuses to reuse `pre_routing_results` when the live payment no longer matches —
/// e.g. the customer changed the amount after the SDK's browse calls evaluated it.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PreRoutingFingerprint {
    pub amount: common_utils::types::MinorUnit,
    pub currency: Option<common_enums::Currency>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(from = "PaymentRoutingInfoSerde", into = "PaymentRoutingInfoSerde")]
pub struct PaymentRoutingInfo {
    pub algorithm: Option<routing::StraightThroughAlgorithm>,
    pub pre_routing_results:
        Option<HashMap<api_enums::PaymentMethodType, PreRoutingConnectorChoice>>,
    /// `None` on blobs written before fingerprinting existed: those reuse as before.
    pub pre_routing_fingerprint: Option<PreRoutingFingerprint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PreRoutingConnectorChoice {
    Single(routing::RoutableConnectorChoice),
    Multiple(Vec<routing::RoutableConnectorChoice>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentRoutingInfoInner {
    pub algorithm: Option<routing::StraightThroughAlgorithm>,
    pub pre_routing_results:
        Option<HashMap<api_enums::PaymentMethodType, PreRoutingConnectorChoice>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_routing_fingerprint: Option<PreRoutingFingerprint>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PaymentRoutingInfoSerde {
    OnlyAlgorithm(Box<routing::StraightThroughAlgorithm>),
    WithDetails(Box<PaymentRoutingInfoInner>),
}

impl From<PaymentRoutingInfoSerde> for PaymentRoutingInfo {
    fn from(value: PaymentRoutingInfoSerde) -> Self {
        match value {
            PaymentRoutingInfoSerde::OnlyAlgorithm(algo) => Self {
                algorithm: Some(*algo),
                pre_routing_results: None,
                pre_routing_fingerprint: None,
            },
            PaymentRoutingInfoSerde::WithDetails(details) => Self {
                algorithm: details.algorithm,
                pre_routing_results: details.pre_routing_results,
                pre_routing_fingerprint: details.pre_routing_fingerprint,
            },
        }
    }
}

impl From<PaymentRoutingInfo> for PaymentRoutingInfoSerde {
    fn from(value: PaymentRoutingInfo) -> Self {
        Self::WithDetails(Box::new(PaymentRoutingInfoInner {
            algorithm: value.algorithm,
            pre_routing_results: value.pre_routing_results,
            pre_routing_fingerprint: value.pre_routing_fingerprint,
        }))
    }
}
