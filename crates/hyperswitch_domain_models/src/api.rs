use std::{collections::HashSet, fmt::Display};

use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    impl_api_event_type,
};

#[derive(Debug, Eq, PartialEq)]
pub enum ApplicationResponse<R> {
    Json(R),
    StatusOk,
    TextPlain(String),
    JsonForRedirection(api_models::payments::RedirectionResponse),
    Form(Box<RedirectionFormData>),
    PaymentLinkForm(Box<PaymentLinkAction>),
    FileData((Vec<u8>, mime::Mime)),
    JsonWithHeaders((R, Vec<(String, masking::Maskable<String>)>)),
    GenericLinkForm(Box<GenericLinks>),
}

impl<T: ApiEventMetric> ApiEventMetric for ApplicationResponse<T> {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        match self {
            Self::Json(r) => r.get_api_event_type(),
            Self::JsonWithHeaders((r, _)) => r.get_api_event_type(),
            _ => None,
        }
    }
}

impl_api_event_type!(Miscellaneous, (PaymentLinkFormData, GenericLinkFormData));

#[derive(Debug, Eq, PartialEq)]
pub struct RedirectionFormData {
    pub redirect_form: crate::router_response_types::RedirectForm,
    pub payment_method_data: Option<api_models::payments::PaymentMethodData>,
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum PaymentLinkAction {
    PaymentLinkFormData(PaymentLinkFormData),
    PaymentLinkStatus(PaymentLinkStatusData),
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkFormData {
    pub js_script: String,
    pub css_script: String,
    pub sdk_url: String,
    pub html_meta_tags: String,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkStatusData {
    pub js_script: String,
    pub css_script: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct GenericLinks {
    pub allowed_domains: HashSet<String>,
    pub data: GenericLinksData,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GenericLinksData {
    ExpiredLink(GenericExpiredLinkData),
    PaymentMethodCollect(GenericLinkFormData),
    PayoutLink(GenericLinkFormData),
    PayoutLinkStatus(GenericLinkStatusData),
    PaymentMethodCollectStatus(GenericLinkStatusData),
}

impl Display for GenericLinksData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::ExpiredLink(_) => "ExpiredLink",
                Self::PaymentMethodCollect(_) => "PaymentMethodCollect",
                Self::PayoutLink(_) => "PayoutLink",
                Self::PayoutLinkStatus(_) => "PayoutLinkStatus",
                Self::PaymentMethodCollectStatus(_) => "PaymentMethodCollectStatus",
            }
        )
    }
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericExpiredLinkData {
    pub title: String,
    pub message: String,
    pub theme: String,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericLinkFormData {
    pub js_data: String,
    pub css_data: String,
    pub sdk_url: String,
    pub html_meta_tags: String,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericLinkStatusData {
    pub js_data: String,
    pub css_data: String,
}
