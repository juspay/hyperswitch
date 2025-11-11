use common_enums::enums::{CavvAlgorithm, Eci, ExemptionIndicator, TransactionStatus};
use euclid::frontend::dir::enums::{
    CustomerDeviceDisplaySize, CustomerDevicePlatform, CustomerDeviceType,
};
use masking::Secret;
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

/// Represents external 3DS authentication data used in the payment flow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct ExternalThreeDsData {
    /// Contains the authentication cryptogram data (CAVV or TAVV).
    #[schema(value_type = Option<Cryptogram>)]
    pub authentication_cryptogram: Cryptogram,
    /// Directory Server Transaction ID generated during the 3DS process.
    #[schema(value_type = Option<String>)]
    pub ds_trans_id: String,
    /// The version of the 3DS protocol used (e.g., "2.1.0" or "2.2.0").
    #[schema(value_type = Option<String>)]
    pub version: String,
    /// Electronic Commerce Indicator (ECI) value representing the 3DS authentication result.
    #[schema(value_type = Option<Eci>)]
    pub eci: Eci,
    /// Indicates the transaction status from the 3DS authentication flow.
    #[schema(value_type = Option<TransactionStatus>)]
    pub transaction_status: TransactionStatus,
    /// Optional exemption indicator specifying the exemption type, if any, used in this transaction.
    #[schema(value_type = Option<ExemptionIndicator>)]
    pub exemption_indicator: Option<ExemptionIndicator>,
    /// Optional network-specific parameters that may be required by certain card networks.
    #[schema(value_type = NetworkParams)]
    pub network_params: Option<NetworkParams>,
}

/// Represents the 3DS cryptogram data returned after authentication.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cryptogram {
    /// Cardholder Authentication Verification Value (CAVV) cryptogram.
    Cavv {
        /// The authentication cryptogram provided by the issuer or ACS.
        #[schema(value_type = Option<String>)]
        authentication_cryptogram: Secret<String>,
    },
    /// Token Authentication Verification Value (TAVV) cryptogram for network token transactions.
    Tavv {
        /// The token authentication cryptogram used for tokenized cards.
        #[schema(value_type = Option<String>)]
        token_authentication_cryptogram: Secret<String>,
    },
}

/// Represents additional network-level parameters for 3DS processing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct NetworkParams {
    /// Parameters specific to Cartes Bancaires network, if applicable.
    #[schema(value_type = Option<CartesBancairesParams>)]
    pub cartes_bancaires: Option<CartesBancairesParams>,
}

/// Represents network-specific parameters for the Cartes Bancaires 3DS process.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CartesBancairesParams {
    /// The algorithm used to generate the CAVV value.
    #[schema(value_type = Option<CavvAlgorithm>)]
    pub cavv_algorithm: CavvAlgorithm,
    /// Exemption indicator specific to Cartes Bancaires network (e.g., "low_value", "trusted_merchant")
    #[schema(value_type = String)]
    pub cb_exemption: String,
    /// Cartes Bancaires risk score assigned during 3DS authentication.
    #[schema(value_type = i32)]
    pub cb_score: i32,
}
