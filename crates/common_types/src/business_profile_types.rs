//! Business profile related types shared across request/response and database types

use std::collections::HashSet;

use hyperswitch_masking::Secret;

/// Webhook details for a business profile
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct WebhookDetails {
    /// Webhook version
    pub webhook_version: Option<String>,
    /// Webhook username
    pub webhook_username: Option<String>,
    /// Webhook password
    pub webhook_password: Option<Secret<String>>,
    /// Webhook URL
    pub webhook_url: Option<Secret<String>>,
    /// Payment created enabled
    pub payment_created_enabled: Option<bool>,
    /// Payment succeeded enabled
    pub payment_succeeded_enabled: Option<bool>,
    /// Payment failed enabled
    pub payment_failed_enabled: Option<bool>,
    /// Payment statuses enabled
    pub payment_statuses_enabled: Option<Vec<common_enums::IntentStatus>>,
    /// Refund statuses enabled
    pub refund_statuses_enabled: Option<Vec<common_enums::RefundStatus>>,
    /// Payout statuses enabled
    pub payout_statuses_enabled: Option<Vec<common_enums::PayoutStatus>>,
    /// Multiple webhooks list
    pub multiple_webhooks_list: Option<Vec<MultipleWebhookDetail>>,
}

common_utils::impl_to_sql_from_sql_json!(WebhookDetails);

/// Detail for a single webhook endpoint
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MultipleWebhookDetail {
    /// Webhook endpoint ID
    pub webhook_endpoint_id: common_utils::id_type::WebhookEndpointId,
    /// Webhook URL
    pub webhook_url: Secret<String>,
    /// Events
    pub events: HashSet<common_enums::EventType>,
    /// Status
    pub status: common_enums::OutgoingWebhookEndpointStatus,
}

/// Authentication connector details
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct AuthenticationConnectorDetails {
    /// Authentication connectors
    pub authentication_connectors: Vec<common_enums::AuthenticationConnectors>,
    /// Three DS requestor URL
    pub three_ds_requestor_url: String,
    /// Three DS requestor app URL
    pub three_ds_requestor_app_url: Option<String>,
}

common_utils::impl_to_sql_from_sql_json!(AuthenticationConnectorDetails);

/// External vault connector details
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct ExternalVaultConnectorDetails {
    /// Vault connector ID
    pub vault_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    /// Vault SDK
    pub vault_sdk: Option<common_enums::VaultSdk>,
    /// Vault token selector
    pub vault_token_selector: Option<Vec<VaultTokenField>>,
}

/// Vault token field
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct VaultTokenField {
    /// Token type
    pub token_type: common_enums::VaultTokenType,
}

common_utils::impl_to_sql_from_sql_json!(ExternalVaultConnectorDetails);

/// Card testing guard configuration
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct CardTestingGuardConfig {
    /// Is card IP blocking enabled
    pub is_card_ip_blocking_enabled: bool,
    /// Card IP blocking threshold
    pub card_ip_blocking_threshold: i32,
    /// Is guest user card blocking enabled
    pub is_guest_user_card_blocking_enabled: bool,
    /// Guest user card blocking threshold
    pub guest_user_card_blocking_threshold: i32,
    /// Is customer ID blocking enabled
    pub is_customer_id_blocking_enabled: bool,
    /// Customer ID blocking threshold
    pub customer_id_blocking_threshold: i32,
    /// Card testing guard expiry
    pub card_testing_guard_expiry: i32,
}

common_utils::impl_to_sql_from_sql_json!(CardTestingGuardConfig);

impl Default for CardTestingGuardConfig {
    fn default() -> Self {
        Self {
            is_card_ip_blocking_enabled: common_utils::consts::DEFAULT_CARD_IP_BLOCKING_STATUS,
            card_ip_blocking_threshold: common_utils::consts::DEFAULT_CARD_IP_BLOCKING_THRESHOLD,
            is_guest_user_card_blocking_enabled:
                common_utils::consts::DEFAULT_GUEST_USER_CARD_BLOCKING_STATUS,
            guest_user_card_blocking_threshold:
                common_utils::consts::DEFAULT_GUEST_USER_CARD_BLOCKING_THRESHOLD,
            is_customer_id_blocking_enabled:
                common_utils::consts::DEFAULT_CUSTOMER_ID_BLOCKING_STATUS,
            customer_id_blocking_threshold:
                common_utils::consts::DEFAULT_CUSTOMER_ID_BLOCKING_THRESHOLD,
            card_testing_guard_expiry:
                common_utils::consts::DEFAULT_CARD_TESTING_GUARD_EXPIRY_IN_SECS,
        }
    }
}

/// Business payout link configuration
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct BusinessPayoutLinkConfig {
    /// Configuration
    #[serde(flatten)]
    pub config: BusinessGenericLinkConfig,
    /// Form layout
    pub form_layout: Option<common_enums::UIWidgetFormLayout>,
    /// Payout test mode
    pub payout_test_mode: Option<bool>,
}

/// Business generic link configuration
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct BusinessGenericLinkConfig {
    /// Domain name
    pub domain_name: Option<String>,
    /// Allowed domains
    pub allowed_domains: HashSet<String>,
    /// UI configuration
    #[serde(flatten)]
    pub ui_config: common_utils::link_utils::GenericLinkUiConfig,
}

common_utils::impl_to_sql_from_sql_json!(BusinessPayoutLinkConfig);

/// Revenue recovery algorithm data
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct RevenueRecoveryAlgorithmData {
    /// Monitoring configured timestamp
    pub monitoring_configured_timestamp: time::PrimitiveDateTime,
}

impl RevenueRecoveryAlgorithmData {
    /// Check if the monitoring threshold has been exceeded
    pub fn has_exceeded_monitoring_threshold(&self, monitoring_threshold_in_seconds: i64) -> bool {
        let total_threshold_time = self.monitoring_configured_timestamp
            + time::Duration::seconds(monitoring_threshold_in_seconds);
        common_utils::date_time::now() >= total_threshold_time
    }
}

common_utils::impl_to_sql_from_sql_json!(RevenueRecoveryAlgorithmData);

/// Configuration for payment method blocking based on card attributes
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct PaymentMethodBlockingConfig {
    /// Card blocking configuration
    pub card: Option<CardBlockingConfig>,
}

/// Card-specific blocking configuration
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct CardBlockingConfig {
    /// Set of issuing countries to block using ISO 3166-1 alpha-2 codes (e.g., ["IN", "US"])
    pub issuing_country: Option<HashSet<common_enums::CountryAlpha2>>,
    /// Set of card types to block (e.g., ["Credit", "Debit"])
    pub card_types: Option<HashSet<common_enums::CardType>>,
    /// Set of card subtypes to block
    pub card_subtypes: Option<HashSet<common_enums::CardSubtype>>,
    /// Set of card issuers to block (e.g., ["HDFC Bank", "ICICI Bank"])
    pub issuers: Option<HashSet<String>>,
    /// Whether to block if BIN is provided but no matching record found in cards_info table.
    /// Defaults to false (allow payment if BIN not found in database).
    pub block_if_bin_info_unavailable: Option<bool>,
}

impl CardBlockingConfig {
    /// Check if payment should be blocked when BIN info is unavailable
    pub fn should_block_if_bin_info_unavailable(&self) -> bool {
        self.block_if_bin_info_unavailable.unwrap_or(false)
    }

    /// Check if a given attribute should be blocked
    pub fn should_block_by_attribute<T>(blocked: &Option<HashSet<T>>, value: Option<&str>) -> bool
    where
        T: std::str::FromStr + std::hash::Hash + Eq,
    {
        blocked
            .as_ref()
            .zip(value)
            .and_then(|(set, s)| s.parse::<T>().ok().map(|v| (set, v)))
            .is_some_and(|(set, v)| set.contains(&v))
    }
}

common_utils::impl_to_sql_from_sql_json!(PaymentMethodBlockingConfig);

pub use crate::payment_link::{
    BusinessPaymentLinkConfig, PaymentLinkBackgroundImageConfig, PaymentLinkConfigRequest,
};
