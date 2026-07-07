//! Payment link specific types

use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    impl_api_event_type,
};

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkFormData {
    pub js_script: String,
    pub css_script: String,
    pub sdk_url: url::Url,
    pub html_meta_tags: String,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkStatusData {
    pub js_script: String,
    pub css_script: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreloadSDKParams {
    pub payment_methods_list: Option<serde_json::Value>,
    pub customer_methods_list: Option<serde_json::Value>,
    pub session_tokens: Option<serde_json::Value>,
    pub blocked_bins: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkPreviewConfig {
    pub test_mode: Option<bool>,
    pub preload_sdk_with_params: Option<PreloadSDKParams>,
    #[serde(flatten)]
    pub payment_link_details: api_models::payments::PaymentLinkDetails,
}

impl_api_event_type!(Miscellaneous, (PaymentLinkFormData, PaymentLinkStatusData));
