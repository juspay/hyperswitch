use common_utils::events::ApiEventMetric;
use hyperswitch_masking::Secret;
use serde_json::{Map, Value};
use superposition_types::Config;

use crate::enums as api_enums;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkConfigRequest {
    pub client_secret: Option<Secret<String>>,
}

impl ApiEventMetric for SdkConfigRequest {}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodCriteria {
    CardNetwork,
    BankName,
    PaymentExperience,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkCriteriaRule {
    pub criteria_value: String,
    pub eligible_connectors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkPaymentMethodType {
    pub payment_method_type: api_enums::PaymentMethodType,
    pub payment_method_criteria: Option<PaymentMethodCriteria>,
    pub criteria_rules: Vec<SdkCriteriaRule>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SdkPaymentMethod {
    pub payment_method: api_enums::PaymentMethod,
    pub payment_method_types: Vec<SdkPaymentMethodType>,
}

/// Tells the SDK whether to tokenize payment method details before calling confirm.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VaultingAction {
    /// Tokenize payment method details through the modular vaulting flow.
    Tokenize,
    /// Skip tokenization and call confirm directly.
    Skip,
}

/// Profile level configuration surfaced to the SDK.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfileAccountConfig {
    pub collect_shipping_details_from_wallet_connector: bool,
    pub collect_billing_details_from_wallet_connector: bool,
    pub always_collect_billing_details_from_wallet_connector: bool,
    pub always_collect_shipping_details_from_wallet_connector: bool,
    /// Whether the SDK should tokenize payment method details before calling confirm.
    pub vaulting_action: VaultingAction,
}

/// Account level configuration surfaced to the SDK.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AccountConfig {
    pub profile: ProfileAccountConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SuperPositionConfigResponse {
    pub raw_configs: Config,
    pub resolved_configs: Option<Map<String, Value>>,
    pub context_used: Map<String, Value>,
    pub payment_methods: Option<Vec<SdkPaymentMethod>>,
    pub account_config: AccountConfig,
}

impl ApiEventMetric for SuperPositionConfigResponse {}
