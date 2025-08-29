use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use wasm_bindgen::prelude::*;

// Simplified types to avoid heavy dependencies
pub type StringMajorUnit = String;

// PaymentLinkDetails struct definition (copied from api_models to avoid heavy dependencies)
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PaymentLinkDetails {
    pub amount: StringMajorUnit,
    pub currency: String, // Simplified from api_enums::Currency
    pub pub_key: String,
    pub client_secret: String,
    pub payment_id: String,     // Simplified from id_type::PaymentId
    pub session_expiry: String, // Simplified from PrimitiveDateTime
    pub merchant_logo: String,
    pub return_url: String,
    pub merchant_name: String,
    pub order_details: Option<Vec<OrderDetailsWithStringAmount>>,
    pub max_items_visible_after_collapse: i8,
    pub theme: String,
    pub merchant_description: Option<String>,
    pub sdk_layout: String,
    pub display_sdk_only: bool,
    pub hide_card_nickname_field: bool,
    pub show_card_form_by_default: bool,
    pub locale: Option<String>,
    pub transaction_details: Option<Vec<PaymentLinkTransactionDetails>>,
    pub background_image: Option<PaymentLinkBackgroundImageConfig>,
    pub details_layout: Option<String>, // Simplified from api_enums::PaymentLinkDetailsLayout
    pub branding_visibility: Option<bool>,
    pub payment_button_text: Option<String>,
    pub skip_status_screen: Option<bool>,
    pub custom_message_for_card_terms: Option<String>,
    pub payment_button_colour: Option<String>,
    pub payment_button_text_colour: Option<String>,
    pub background_colour: Option<String>,
    pub sdk_ui_rules: Option<HashMap<String, HashMap<String, String>>>,
    pub status: String, // Simplified from api_enums::IntentStatus
    pub enable_button_only_on_form_ready: bool,
    pub payment_form_header_text: Option<String>,
    pub payment_form_label_type: Option<String>, // Simplified from api_enums::PaymentLinkSdkLabelType
    pub show_card_terms: Option<String>, // Simplified from api_enums::PaymentLinkShowSdkTerms
    pub is_setup_mandate_flow: Option<bool>,
    pub capture_method: Option<String>, // Simplified from common_enums::CaptureMethod
    pub setup_future_usage_applied: Option<String>, // Simplified from common_enums::FutureUsage
    pub color_icon_card_cvc_error: Option<String>,
}

// Supporting structs (simplified versions)
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct OrderDetailsWithStringAmount {
    pub product_name: String,
    pub quantity: u16,
    pub amount: StringMajorUnit,
    pub product_img_link: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PaymentLinkTransactionDetails {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PaymentLinkBackgroundImageConfig {
    pub url: String,
}

#[wasm_bindgen]
pub fn generate_payment_link_preview(config_json: &str) -> Result<String, JsValue> {
    // Step 1: Read string/JSON input
    // Step 2: Parse it to required type (this validates the format)
    let config: PaymentLinkDetails = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?;

    // Step 3: Convert it back to string using serialize
    let config_str = serde_json::to_string(&config)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize config: {}", e)))?;

    // Use the serialized config directly for template injection
    let url_encoded_str = urlencoding::encode(&config_str);
    let js_script = format!("window.__PAYMENT_DETAILS = '{}';", url_encoded_str);

    // Extract values for template processing
    let theme = &config.theme;
    let merchant_name = &config.merchant_name;

    // Generate CSS with color scheme and custom rules
    let mut css_script = format!(":root {{ --primary-color: {}; }}", theme);

    // Add custom SDK UI rules if provided
    if let Some(sdk_ui_rules) = &config.sdk_ui_rules {
        css_script.push_str("\n/* Custom SDK UI Rules */\n");
        for (selector, styles) in sdk_ui_rules {
            css_script.push_str(&format!("{} {{\n", selector));
            for (property, value) in styles {
                // Convert camelCase to kebab-case
                let css_property = camel_to_kebab(property);
                css_script.push_str(&format!("  {}: {};\n", css_property, value));
            }
            css_script.push_str("}\n");
        }
    }

    // Generate meta tags
    let meta_tags = format!(
        r#"<meta property="og:title" content="Payment request from {}"/>
        <meta property="og:description" content="Preview payment link"/>"#,
        merchant_name
    );

    // Load embedded templates (same as backend uses)
    let html_template =
        include_str!("../../router/src/core/payment_link/payment_link_initiate/payment_link.html");
    let css_template =
        include_str!("../../router/src/core/payment_link/payment_link_initiate/payment_link.css");
    let js_template =
        include_str!("../../router/src/core/payment_link/payment_link_initiate/payment_link.js");
    let initiator_js = include_str!(
        "../../router/src/core/payment_link/payment_link_initiate/payment_link_initiator.js"
    );
    let locale_js = include_str!("../../router/src/core/payment_link/locale.js");

    // Use Tera for CSS template processing
    let mut css_tera = Tera::default();
    css_tera
        .add_raw_template("payment_link_css", css_template)
        .map_err(|e| JsValue::from_str(&format!("Failed to add CSS template: {}", e)))?;

    let mut css_context = Context::new();
    css_context.insert("css_color_scheme", &css_script);

    let rendered_css = css_tera
        .render("payment_link_css", &css_context)
        .map_err(|e| JsValue::from_str(&format!("Failed to render CSS template: {}", e)))?;

    // Use Tera for JS template processing
    let mut js_tera = Tera::default();
    js_tera
        .add_raw_template("payment_link_js", js_template)
        .map_err(|e| JsValue::from_str(&format!("Failed to add JS template: {}", e)))?;

    let mut js_context = Context::new();
    js_context.insert("payment_details_js_script", &js_script);
    js_context.insert("sdk_origin", "http://localhost:9050"); // Default SDK origin for preview

    let rendered_js = js_tera
        .render("payment_link_js", &js_context)
        .map_err(|e| JsValue::from_str(&format!("Failed to render JS template: {}", e)))?;

    // Generate preload links for SDK
    let preload_links = r#"<link rel="preload" href="https://fonts.googleapis.com/css2?family=Montserrat:wght@400;500;600;700;800" as="style">
            <link rel="preload" href="http://localhost:9050/HyperLoader.js" as="script">"#;

    // Generate SDK script tag
    let sdk_script =
        r#"<script src="http://localhost:9050/HyperLoader.js" onload="initializeSDK()"></script>"#;

    // Logging template (empty for preview)
    let logging_template = "";

    // Use Tera for template processing (consistent with backend approach)
    let mut tera = Tera::default();
    tera.add_raw_template("payment_link", html_template)
        .map_err(|e| JsValue::from_str(&format!("Failed to add template: {}", e)))?;

    let mut context = Context::new();
    context.insert("rendered_meta_tag_html", &meta_tags);
    context.insert("preload_link_tags", preload_links);
    context.insert("hyperloader_sdk_link", sdk_script);
    context.insert("locale_template", locale_js);
    context.insert("rendered_css", &rendered_css);
    context.insert("rendered_js", &rendered_js);
    context.insert("payment_link_initiator", initiator_js);
    context.insert("logging_template", logging_template);

    let final_html = tera
        .render("payment_link", &context)
        .map_err(|e| JsValue::from_str(&format!("Failed to render template: {}", e)))?;

    Ok(final_html)
}

// Helper function to convert camelCase to kebab-case
fn camel_to_kebab(s: &str) -> String {
    let mut result = String::new();
    if s.is_empty() {
        return result;
    }

    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            let should_add_dash = i > 0
                && (chars.get(i - 1).map(|c| c.is_lowercase()).unwrap_or(false)
                    || (i + 1 < chars.len()
                        && chars.get(i + 1).map(|c| c.is_lowercase()).unwrap_or(false)
                        && chars.get(i - 1).map(|c| c.is_uppercase()).unwrap_or(false)));

            if should_add_dash {
                result.push('-');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

#[wasm_bindgen]
pub fn validate_payment_link_config(config_json: &str) -> Result<String, JsValue> {
    // Parse the configuration using the proper type
    let config: PaymentLinkDetails = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Validate theme color
    if !config.theme.starts_with('#') || config.theme.len() != 7 {
        errors.push("Theme color must be a valid hex color (e.g., #4E6ADD)".to_string());
    }

    // Validate background color
    if let Some(bg_color) = &config.background_colour {
        if !bg_color.starts_with('#') || bg_color.len() != 7 {
            errors.push("Background color must be a valid hex color (e.g., #FFFFFF)".to_string());
        }
    }

    // Validate merchant logo URL
    if !config.merchant_logo.is_empty() && !config.merchant_logo.starts_with("http") {
        warnings.push("Merchant logo should be a valid HTTP/HTTPS URL".to_string());
    }

    // Validate SDK layout
    let valid_layouts = ["accordion", "tabs", "spaced_accordion"];
    if !valid_layouts.contains(&config.sdk_layout.as_str()) {
        errors.push(format!(
            "SDK layout must be one of: {}",
            valid_layouts.join(", ")
        ));
    }

    // Validate required fields
    if config.amount.is_empty() {
        errors.push("Amount is required".to_string());
    }

    if config.currency.is_empty() {
        errors.push("Currency is required".to_string());
    }

    if config.merchant_name.is_empty() {
        errors.push("Merchant name is required".to_string());
    }

    if config.payment_id.is_empty() {
        errors.push("Payment ID is required".to_string());
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
