use euclid::frontend::dir::enums::{
    CustomerDeviceDisplaySize, CustomerDevicePlatform, CustomerDeviceType,
};
use utoipa::ToSchema;

/// Represents the payment data used in the 3DS decision rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentData {
    /// The amount of the payment in minor units (e.g., cents for USD).
    #[schema(value_type = i64)]
    pub amount: common_utils::types::MinorUnit,
    /// The currency of the payment.
    #[schema(value_type = Currency)]
    pub currency: common_enums::Currency,
}

/// Represents metadata about the payment method used in the 3DS decision rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PaymentMethodMetaData {
    /// The card network (e.g., Visa, Mastercard) if the payment method is a card.
    #[schema(value_type = CardNetwork)]
    pub card_network: Option<common_enums::CardNetwork>,
}

/// Represents data about the customer's device used in the 3DS decision rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CustomerDeviceData {
    /// The platform of the customer's device (e.g., Web, Android, iOS).
    pub platform: Option<CustomerDevicePlatform>,
    /// The type of the customer's device (e.g., Mobile, Tablet, Desktop).
    pub device_type: Option<CustomerDeviceType>,
    /// The display size of the customer's device.
    pub display_size: Option<CustomerDeviceDisplaySize>,
}

/// Represents data about the issuer used in the 3DS decision rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct IssuerData {
    /// The name of the issuer.
    pub name: Option<String>,
    /// The country of the issuer.
    #[schema(value_type = Country)]
    pub country: Option<common_enums::Country>,
}

/// Represents data about the acquirer used in the 3DS decision rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AcquirerData {
    /// The country of the acquirer.
    #[schema(value_type = Country)]
    pub country: Option<common_enums::Country>,
    /// The fraud rate associated with the acquirer.
    pub fraud_rate: Option<f64>,
}

/// Represents the request to execute a 3DS decision rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ThreeDsDecisionRuleExecuteRequest {
    /// The ID of the routing algorithm to be executed.
    #[schema(value_type = String)]
    pub routing_id: common_utils::id_type::RoutingId,
    /// Data related to the payment.
    pub payment: PaymentData,
    /// Optional metadata about the payment method.
    pub payment_method: Option<PaymentMethodMetaData>,
    /// Optional data about the customer's device.
    pub customer_device: Option<CustomerDeviceData>,
    /// Optional data about the issuer.
    pub issuer: Option<IssuerData>,
    /// Optional data about the acquirer.
    pub acquirer: Option<AcquirerData>,
}

/// Represents the response from executing a 3DS decision rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ThreeDsDecisionRuleExecuteResponse {
    /// The decision made by the 3DS decision rule engine.
    #[schema(value_type = ThreeDSDecision)]
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
