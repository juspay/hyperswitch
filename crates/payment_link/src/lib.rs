pub mod css_generator;
pub mod js_generator;
pub mod meta_tags;
pub mod template_renderer;
pub mod types;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use css_generator::get_css_script;
pub use js_generator::get_js_script;
pub use meta_tags::get_meta_tags_html;
pub use template_renderer::{
    build_payment_link_html, build_secure_payment_link_html, get_payment_link_status,
};
pub use types::{PaymentLinkFormData, PaymentLinkStatusData};
// WASM bindings - thin wrappers around implementation functions in wasm.rs
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

/// Generate a payment link HTML preview from configuration JSON
///
/// This function is exported to JavaScript when compiled as WASM.
/// It wraps the implementation function in wasm.rs.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn generate_payment_link_preview(config_json: &str) -> Result<String, JsValue> {
    wasm::generate_payment_link_preview_impl(config_json).map_err(|e| JsValue::from_str(&e))
}

/// Validate payment link configuration and return validation results as JSON
///
/// This function is exported to JavaScript when compiled as WASM.
/// It wraps the implementation function in wasm.rs.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn validate_payment_link_config(config_json: &str) -> Result<String, JsValue> {
    wasm::validate_payment_link_config_impl(config_json).map_err(|e| JsValue::from_str(&e))
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::js_generator::convert_custom_message_keys_to_camel;
    use serde_json::json;

    #[test]
    fn test_camel_case_conversion_for_test_mode() {
        let mut value = json!({
            "test_mode": true,
            "client_secret": "secret_abc123"
        });

        convert_custom_message_keys_to_camel(&mut value);

        assert!(
            value.get("testMode").is_some(),
            "test_mode should be converted to testMode"
        );
        assert!(
            value.get("test_mode").is_none(),
            "test_mode should be removed"
        );
        assert_eq!(value["testMode"], json!(true));
    }

    #[test]
    fn test_camel_case_conversion_for_preload_sdk_with_params() {
        let mut value = json!({
            "preload_sdk_with_params": {
                "payment_methods_list": ["card", "wallet"]
            },
            "client_secret": "secret_abc123"
        });

        convert_custom_message_keys_to_camel(&mut value);

        assert!(
            value.get("preloadSDKWithParams").is_some(),
            "preload_sdk_with_params should be converted to preloadSDKWithParams"
        );
        assert!(
            value.get("preload_sdk_with_params").is_none(),
            "preload_sdk_with_params should be removed"
        );
        assert!(
            value["preloadSDKWithParams"]
                .get("paymentMethodsList")
                .is_some(),
            "Nested payment_methods_list should be converted to paymentMethodsList"
        );
    }

    #[test]
    fn test_backward_compatibility_without_new_fields() {
        let mut value = json!({
            "client_secret": "secret_abc123",
            "merchant_name": "Test Merchant"
        });

        convert_custom_message_keys_to_camel(&mut value);

        assert!(
            value.get("client_secret").is_some(),
            "client_secret should remain unchanged"
        );
        assert!(
            value.get("merchant_name").is_some(),
            "merchant_name should remain unchanged"
        );
    }

    #[test]
    fn test_all_new_fields_together() {
        let mut value = json!({
            "test_mode": true,
            "preload_sdk_with_params": {
                "payment_methods_list": ["card"],
                "customer_methods_list": ["upi"],
                "session_tokens": ["token1"],
                "blocked_bins": ["411111"]
            },
            "client_secret": "secret_abc123"
        });

        convert_custom_message_keys_to_camel(&mut value);

        assert!(value.get("testMode").is_some(), "Should have testMode");
        assert!(
            value.get("preloadSDKWithParams").is_some(),
            "Should have preloadSDKWithParams"
        );
        let preload = &value["preloadSDKWithParams"];
        assert!(
            preload.get("paymentMethodsList").is_some(),
            "Should have paymentMethodsList"
        );
        assert!(
            preload.get("customerMethodsList").is_some(),
            "Should have customerMethodsList"
        );
        assert!(
            preload.get("sessionTokens").is_some(),
            "Should have sessionTokens"
        );
        assert!(
            preload.get("blockedBins").is_some(),
            "Should have blockedBins"
        );
    }
}
