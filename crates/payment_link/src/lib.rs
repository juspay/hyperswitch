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
