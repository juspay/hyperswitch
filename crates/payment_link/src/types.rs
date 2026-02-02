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

impl_api_event_type!(Miscellaneous, (PaymentLinkFormData, PaymentLinkStatusData));
