use std::{collections::HashSet, fmt::Display};

use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    impl_api_event_type,
};

use super::payment_method_data::PaymentMethodData;

#[derive(Debug, PartialEq)]
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

impl<R> ApplicationResponse<R> {
    /// Get the json response from response
    #[inline]
    pub fn get_json_body(
        self,
    ) -> common_utils::errors::CustomResult<R, common_utils::errors::ValidationError> {
        match self {
            Self::Json(body) | Self::JsonWithHeaders((body, _)) => Ok(body),
            Self::TextPlain(_)
            | Self::JsonForRedirection(_)
            | Self::Form(_)
            | Self::PaymentLinkForm(_)
            | Self::FileData(_)
            | Self::GenericLinkForm(_)
            | Self::StatusOk => Err(common_utils::errors::ValidationError::InvalidValue {
                message: "expected either Json or JsonWithHeaders Response".to_string(),
            }
            .into()),
        }
    }
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

impl_api_event_type!(Miscellaneous, (GenericLinkFormData));

#[derive(Debug, PartialEq)]
pub struct RedirectionFormData {
    pub redirect_form: crate::router_response_types::RedirectForm,
    pub payment_method_data: Option<PaymentMethodData>,
    pub amount: String,
    pub currency: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum PaymentLinkAction {
    PaymentLinkFormData(payment_link::PaymentLinkFormData),
    PaymentLinkStatus(payment_link::PaymentLinkStatusData),
}

#[derive(Debug, Eq, PartialEq)]
pub struct GenericLinks {
    pub allowed_domains: HashSet<String>,
    pub data: GenericLinksData,
    pub locale: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GenericLinksData {
    ExpiredLink(GenericExpiredLinkData),
    PaymentMethodCollect(GenericLinkFormData),
    PayoutLink(GenericLinkFormData),
    PayoutLinkStatus(GenericLinkStatusData),
    PaymentMethodCollectStatus(GenericLinkStatusData),
    SecurePaymentLink(payment_link::PaymentLinkFormData),
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
                Self::SecurePaymentLink(_) => "SecurePaymentLink",
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
    pub sdk_url: url::Url,
    pub html_meta_tags: String,
}

#[derive(Debug, Eq, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenericLinkStatusData {
    pub js_data: String,
    pub css_data: String,
}
