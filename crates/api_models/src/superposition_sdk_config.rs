use common_utils::events::ApiEventMetric;
use serde_json::{Map, Value};
use superposition_types::Config;

use crate::enums as api_enums;

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

/// Tells the SDK which vault strategy to use before collecting card details.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SdkVaultStrategy {
    /// Hyperswitch vault — our own SDK, load the card form fast.
    Internal,
    /// Non-Hyperswitch vault (e.g. VGS) — wait for the `/session` call to fetch creds before
    /// loading.
    External,
    /// Skip vaulting and call confirm directly. Used when payment-method-modular is not allowed,
    /// or when external vault is enabled but no vault SDK is configured.
    Skip,
}

/// Profile level configuration surfaced to the SDK.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfileAccountConfig {
    pub collect_shipping_details_from_wallet_connector: bool,
    pub collect_billing_details_from_wallet_connector: bool,
    pub always_collect_billing_details_from_wallet_connector: bool,
    pub always_collect_shipping_details_from_wallet_connector: bool,
    /// Which vault strategy the SDK should use before collecting card details.
    pub vault_sdk: SdkVaultStrategy,
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
