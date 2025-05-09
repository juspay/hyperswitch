use euclid::frontend::dir::enums::{
    CustomerDeviceDisplaySize, CustomerDevicePlatform, CustomerDeviceType,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentData {
    pub amount: common_utils::types::MinorUnit,
    pub currency: common_enums::Currency,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentMethodData {
    pub card_network: Option<common_enums::CardNetwork>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomerDeviceData {
    pub platform: Option<CustomerDevicePlatform>,
    pub device_type: Option<CustomerDeviceType>,
    pub display_size: Option<CustomerDeviceDisplaySize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IssuerData {
    pub name: Option<String>,
    pub country: Option<common_enums::Country>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AcquirerData {
    pub country: Option<common_enums::Country>,
    pub fraud_rate: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreeDsDecisionRuleExecuteRequest {
    pub routing_id: common_utils::id_type::RoutingId,
    pub payment: PaymentData,
    pub payment_method: Option<PaymentMethodData>,
    pub customer_device: Option<CustomerDeviceData>,
    pub issuer: Option<IssuerData>,
    pub acquirer: Option<AcquirerData>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreeDsDecisionRuleExecuteResponse {
    pub decision: common_types::three_ds_decision_rule_engine::ThreeDSDecision,
}

impl common_utils::events::ApiEventMetric for ThreeDsDecisionRuleExecuteRequest {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::ThreeDsDecisionRule)
    }
}

impl common_utils::events::ApiEventMetric for ThreeDsDecisionRuleExecuteResponse {
    fn get_api_event_type(&self) -> Option<common_utils::events::ApiEventsType> {
        Some(common_utils::events::ApiEventsType::ThreeDsDecisionRule)
    }
}
