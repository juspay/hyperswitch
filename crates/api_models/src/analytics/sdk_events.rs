use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use super::{NameDescription, TimeRange};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkEventsRequest {
    pub payment_id: String,
    pub time_range: TimeRange,
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct SdkEventFilters {
    #[serde(default)]
    pub payment_method: Vec<String>,
    #[serde(default)]
    pub platform: Vec<String>,
    #[serde(default)]
    pub browser_name: Vec<String>,
    #[serde(default)]
    pub source: Vec<String>,
    #[serde(default)]
    pub component: Vec<String>,
    #[serde(default)]
    pub payment_experience: Vec<String>,
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    strum::AsRefStr,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    strum::Display,
    strum::EnumIter,
    Clone,
    Copy,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SdkEventDimensions {
    // Do not change the order of these enums
    // Consult the Dashboard FE folks since these also affects the order of metrics on FE
    PaymentMethod,
    Platform,
    BrowserName,
    Source,
    Component,
    PaymentExperience,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::AsRefStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SdkEventMetrics {
    PaymentAttempts,
    PaymentSuccessCount,
    PaymentMethodsCallCount,
    SdkRenderedCount,
    SdkInitiatedCount,
    PaymentMethodSelectedCount,
    PaymentDataFilledCount,
    AveragePaymentTime,
}

#[derive(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumIter,
    strum::AsRefStr,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SdkEventNames {
    StripeElementsCalled,
    AppRendered,
    PaymentMethodChanged,
    PaymentDataFilled,
    PaymentAttempt,
    PaymentSuccess,
    PaymentMethodsCall,
    ConfirmCall,
    SessionsCall,
    CustomerPaymentMethodsCall,
    RedirectingUser,
    DisplayBankTransferInfoPage,
    DisplayQrCodeInfoPage,
}

pub mod metric_behaviour {
    pub struct PaymentAttempts;
    pub struct PaymentSuccessCount;
    pub struct PaymentMethodsCallCount;
    pub struct SdkRenderedCount;
    pub struct SdkInitiatedCount;
    pub struct PaymentMethodSelectedCount;
    pub struct PaymentDataFilledCount;
    pub struct AveragePaymentTime;
}

impl From<SdkEventMetrics> for NameDescription {
    fn from(value: SdkEventMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<SdkEventDimensions> for NameDescription {
    fn from(value: SdkEventDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct SdkEventMetricsBucketIdentifier {
    pub payment_method: Option<String>,
    pub platform: Option<String>,
    pub browser_name: Option<String>,
    pub source: Option<String>,
    pub component: Option<String>,
    pub payment_experience: Option<String>,
    pub time_bucket: Option<String>,
}

impl SdkEventMetricsBucketIdentifier {
    pub fn new(
        payment_method: Option<String>,
        platform: Option<String>,
        browser_name: Option<String>,
        source: Option<String>,
        component: Option<String>,
        payment_experience: Option<String>,
        time_bucket: Option<String>,
    ) -> Self {
        Self {
            payment_method,
            platform,
            browser_name,
            source,
            component,
            payment_experience,
            time_bucket,
        }
    }
}

impl Hash for SdkEventMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.payment_method.hash(state);
        self.platform.hash(state);
        self.browser_name.hash(state);
        self.source.hash(state);
        self.component.hash(state);
        self.payment_experience.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for SdkEventMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct SdkEventMetricsBucketValue {
    pub payment_attempts: Option<u64>,
    pub payment_success_count: Option<u64>,
    pub payment_methods_call_count: Option<u64>,
    pub average_payment_time: Option<f64>,
    pub sdk_rendered_count: Option<u64>,
    pub sdk_initiated_count: Option<u64>,
    pub payment_method_selected_count: Option<u64>,
    pub payment_data_filled_count: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: SdkEventMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: SdkEventMetricsBucketIdentifier,
}
