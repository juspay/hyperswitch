use std::collections::{HashMap, HashSet};

use common_enums::{AuthenticationConnectors, UIWidgetFormLayout, VaultSdk};
use masking::Secret;
use time::Duration;


#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct AuthenticationConnectorDetails {
    pub authentication_connectors: Vec<AuthenticationConnectors>,
    pub three_ds_requestor_url: String,
    pub three_ds_requestor_app_url: Option<String>,
}

common_utils::impl_to_sql_from_sql_json!(AuthenticationConnectorDetails);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct ExternalVaultConnectorDetails {
    pub vault_connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub vault_sdk: Option<VaultSdk>,
}

common_utils::impl_to_sql_from_sql_json!(ExternalVaultConnectorDetails);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct CardTestingGuardConfig {
    pub is_card_ip_blocking_enabled: bool,
    pub card_ip_blocking_threshold: i32,
    pub is_guest_user_card_blocking_enabled: bool,
    pub guest_user_card_blocking_threshold: i32,
    pub is_customer_id_blocking_enabled: bool,
    pub customer_id_blocking_threshold: i32,
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


#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Json)]
pub struct WebhookDetails {
    pub webhook_version: Option<String>,
    pub webhook_username: Option<String>,
    pub webhook_password: Option<Secret<String>>,
    pub webhook_url: Option<Secret<String>>,
    pub payment_created_enabled: Option<bool>,
    pub payment_succeeded_enabled: Option<bool>,
    pub payment_failed_enabled: Option<bool>,
    pub payment_statuses_enabled: Option<Vec<common_enums::IntentStatus>>,
    pub refund_statuses_enabled: Option<Vec<common_enums::RefundStatus>>,
    pub payout_statuses_enabled: Option<Vec<common_enums::PayoutStatus>>,
}

common_utils::impl_to_sql_from_sql_json!(WebhookDetails);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct BusinessPaymentLinkConfig {
    pub domain_name: Option<String>,
    #[serde(flatten)]
    pub default_config: Option<PaymentLinkConfigRequest>,
    pub business_specific_configs: Option<HashMap<String, PaymentLinkConfigRequest>>,
    pub allowed_domains: Option<HashSet<String>>,
    pub branding_visibility: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentLinkConfigRequest {
    pub theme: Option<String>,
    pub logo: Option<String>,
    pub seller_name: Option<String>,
    pub sdk_layout: Option<String>,
    pub display_sdk_only: Option<bool>,
    pub enabled_saved_payment_method: Option<bool>,
    pub hide_card_nickname_field: Option<bool>,
    pub show_card_form_by_default: Option<bool>,
    pub background_image: Option<crate::payments::PaymentLinkBackgroundImageConfig>,
    pub details_layout: Option<common_enums::PaymentLinkDetailsLayout>,
    pub payment_button_text: Option<String>,
    pub custom_message_for_card_terms: Option<String>,
    pub payment_button_colour: Option<String>,
    pub skip_status_screen: Option<bool>,
    pub payment_button_text_colour: Option<String>,
    pub background_colour: Option<String>,
    pub sdk_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    pub payment_link_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    pub enable_button_only_on_form_ready: Option<bool>,
    pub payment_form_header_text: Option<String>,
    pub payment_form_label_type: Option<common_enums::PaymentLinkSdkLabelType>,
    pub show_card_terms: Option<common_enums::PaymentLinkShowSdkTerms>,
    pub is_setup_mandate_flow: Option<bool>,
    pub color_icon_card_cvc_error: Option<String>,
}



common_utils::impl_to_sql_from_sql_json!(BusinessPaymentLinkConfig);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct BusinessPayoutLinkConfig {
    #[serde(flatten)]
    pub config: BusinessGenericLinkConfig,
    pub form_layout: Option<UIWidgetFormLayout>,
    pub payout_test_mode: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct BusinessGenericLinkConfig {
    pub domain_name: Option<String>,
    pub allowed_domains: HashSet<String>,
    #[serde(flatten)]
    pub ui_config: common_utils::link_utils::GenericLinkUiConfig,
}

common_utils::impl_to_sql_from_sql_json!(BusinessPayoutLinkConfig);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct RevenueRecoveryAlgorithmData {
    pub monitoring_configured_timestamp: time::PrimitiveDateTime,
}

impl RevenueRecoveryAlgorithmData {
    pub fn has_exceeded_monitoring_threshold(&self, monitoring_threshold_in_seconds: i64) -> bool {
        let total_threshold_time = self.monitoring_configured_timestamp
            + Duration::seconds(monitoring_threshold_in_seconds);
        common_utils::date_time::now() >= total_threshold_time
    }
}

common_utils::impl_to_sql_from_sql_json!(RevenueRecoveryAlgorithmData);
