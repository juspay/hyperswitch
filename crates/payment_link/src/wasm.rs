use api_models::{admin::PaymentLinkConfig, payments::PaymentLinkData};

use crate::{
    build_payment_link_html, get_css_script, get_js_script, get_meta_tags_html, PaymentLinkFormData,
};

const SDK_URL: &str = env!("SDK_URL");

/// Implementation function for generating payment link preview
/// Called by the wasm_bindgen wrapper in lib.rs
pub fn generate_payment_link_preview_impl(config_json: &str) -> Result<String, String> {
    let payment_link_details: api_models::payments::PaymentLinkDetails =
        serde_json::from_str(config_json)
            .map_err(|e| format!("Failed to deserialize PaymentLinkDetails: {}", e))?;

    let mut payment_link_config = PaymentLinkConfig {
        theme: payment_link_details.theme.clone(),
        logo: payment_link_details.merchant_logo.clone(),
        seller_name: payment_link_details.merchant_name.clone(),
        sdk_layout: payment_link_details.sdk_layout.clone(),
        display_sdk_only: payment_link_details.display_sdk_only,
        hide_card_nickname_field: payment_link_details.hide_card_nickname_field,
        show_card_form_by_default: payment_link_details.show_card_form_by_default,
        transaction_details: payment_link_details.transaction_details.clone(),
        background_image: payment_link_details.background_image.clone(),
        details_layout: payment_link_details.details_layout,
        branding_visibility: payment_link_details.branding_visibility,
        payment_button_text: payment_link_details.payment_button_text.clone(),
        custom_message_for_card_terms: payment_link_details.custom_message_for_card_terms.clone(),
        payment_button_colour: payment_link_details.payment_button_colour.clone(),
        skip_status_screen: payment_link_details.skip_status_screen,
        background_colour: payment_link_details.background_colour.clone(),
        payment_button_text_colour: payment_link_details.payment_button_text_colour.clone(),
        sdk_ui_rules: payment_link_details.sdk_ui_rules.clone(),
        enable_button_only_on_form_ready: payment_link_details.enable_button_only_on_form_ready,
        payment_form_header_text: payment_link_details.payment_form_header_text.clone(),
        payment_form_label_type: payment_link_details.payment_form_label_type,
        show_card_terms: payment_link_details.show_card_terms,
        is_setup_mandate_flow: payment_link_details.is_setup_mandate_flow,
        color_icon_card_cvc_error: payment_link_details.color_icon_card_cvc_error.clone(),
        enabled_saved_payment_method: false,
        allowed_domains: None,
        payment_link_ui_rules: None,
        custom_message_for_payment_method_types: payment_link_details
            .custom_message_for_payment_method_types
            .clone(),
    };

    if let Ok(config_from_json) = serde_json::from_str::<PaymentLinkConfig>(config_json) {
        payment_link_config.enabled_saved_payment_method =
            config_from_json.enabled_saved_payment_method;
        payment_link_config.allowed_domains = config_from_json.allowed_domains;
        payment_link_config.payment_link_ui_rules = config_from_json.payment_link_ui_rules;
    }

    let sdk_url = url::Url::parse(SDK_URL).map_err(|e| format!("Invalid SDK URL: {}", e))?;

    let js_script = get_js_script(&PaymentLinkData::PaymentLinkDetails(Box::new(
        payment_link_details.clone(),
    )))
    .map_err(|e| format!("Failed to generate JS script: {:?}", e))?;

    let css_script = get_css_script(&payment_link_config)
        .map_err(|e| format!("Failed to generate CSS script: {:?}", e))?;

    let html_meta_tags = get_meta_tags_html(&payment_link_details);

    let payment_link_form_data = PaymentLinkFormData {
        js_script,
        sdk_url,
        css_script,
        html_meta_tags,
    };

    build_payment_link_html(payment_link_form_data)
        .map_err(|e| format!("Failed to build payment link HTML: {:?}", e))
}

/// Implementation function for validating payment link config
/// Called by the wasm_bindgen wrapper in lib.rs
pub fn validate_payment_link_config_impl(config_json: &str) -> Result<String, String> {
    let config: api_models::payments::PaymentLinkDetails =
        serde_json::from_str(config_json).map_err(|e| format!("Failed to parse config: {}", e))?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if !config.theme.starts_with('#') || config.theme.len() != 7 {
        errors.push("Theme color must be a valid hex color (e.g., #4E6ADD)".to_string());
    }

    if let Some(bg_color) = &config.background_colour {
        if !bg_color.starts_with('#') || bg_color.len() != 7 {
            errors.push("Background color must be a valid hex color (e.g., #FFFFFF)".to_string());
        }
    }

    if !config.merchant_logo.is_empty() && !config.merchant_logo.starts_with("http") {
        warnings.push("Merchant logo should be a valid HTTP/HTTPS URL".to_string());
    }

    let valid_layouts = ["accordion", "tabs", "spaced_accordion"];
    if !valid_layouts.contains(&config.sdk_layout.as_str()) {
        errors.push(format!(
            "SDK layout must be one of: {}",
            valid_layouts.join(", ")
        ));
    }

    if config.merchant_name.is_empty() {
        errors.push("Merchant name is required".to_string());
    }

    if config.client_secret.is_empty() {
        errors.push("Client secret is required".to_string());
    }

    if config.pub_key.is_empty() {
        errors.push("Publishable key is required".to_string());
    }

    let validation_result = serde_json::json!({
        "valid": errors.is_empty(),
        "errors": errors,
        "warnings": warnings
    });

    Ok(validation_result.to_string())
}
