//! Payment link related types shared across request/response and database types

use std::collections::{HashMap, HashSet};

/// Configuration for payment links at the business level
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct BusinessPaymentLinkConfig {
    /// Domain name for the payment link
    pub domain_name: Option<String>,
    /// Default configuration for payment links
    #[serde(flatten)]
    pub default_config: Option<PaymentLinkConfigRequest>,
    /// Business-specific configurations
    pub business_specific_configs: Option<HashMap<String, PaymentLinkConfigRequest>>,
    /// Allowed domains for the payment link
    pub allowed_domains: Option<HashSet<String>>,
    /// Branding visibility flag
    pub branding_visibility: Option<bool>,
}

/// Configuration for payment link request
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PaymentLinkConfigRequest {
    /// Theme for the payment link
    pub theme: Option<String>,
    /// Logo URL
    pub logo: Option<String>,
    /// Seller name
    pub seller_name: Option<String>,
    /// SDK layout
    pub sdk_layout: Option<String>,
    /// Display SDK only
    pub display_sdk_only: Option<bool>,
    /// Enable saved payment method
    pub enabled_saved_payment_method: Option<bool>,
    /// Hide card nickname field
    pub hide_card_nickname_field: Option<bool>,
    /// Show card form by default
    pub show_card_form_by_default: Option<bool>,
    /// Background image configuration
    pub background_image: Option<PaymentLinkBackgroundImageConfig>,
    /// Details layout
    pub details_layout: Option<common_enums::PaymentLinkDetailsLayout>,
    /// Payment button text
    pub payment_button_text: Option<String>,
    /// Custom message for card terms
    pub custom_message_for_card_terms: Option<String>,
    /// Custom message for payment method types
    pub custom_message_for_payment_method_types: Option<crate::payments::PaymentMethodsConfig>,
    /// Payment button colour
    pub payment_button_colour: Option<String>,
    /// Skip status screen
    pub skip_status_screen: Option<bool>,
    /// Payment button text colour
    pub payment_button_text_colour: Option<String>,
    /// Background colour
    pub background_colour: Option<String>,
    /// SDK UI rules
    pub sdk_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    /// Payment link UI rules
    pub payment_link_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    /// Enable button only on form ready
    pub enable_button_only_on_form_ready: Option<bool>,
    /// Payment form header text
    pub payment_form_header_text: Option<String>,
    /// Payment form label type
    pub payment_form_label_type: Option<common_enums::PaymentLinkSdkLabelType>,
    /// Show card terms
    pub show_card_terms: Option<common_enums::PaymentLinkShowSdkTerms>,
    /// Is setup mandate flow
    pub is_setup_mandate_flow: Option<bool>,
    /// Color icon for card CVC error
    pub color_icon_card_cvc_error: Option<String>,
}

/// Background image configuration for payment links
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct PaymentLinkBackgroundImageConfig {
    /// URL for the background image
    pub url: common_utils::types::Url,
    /// Position of the background image
    pub position: Option<common_enums::ElementPosition>,
    /// Size of the background image
    pub size: Option<common_enums::ElementSize>,
}

common_utils::impl_to_sql_from_sql_json!(BusinessPaymentLinkConfig);
