use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use common_utils::id_type;

use super::{ForexMetric, NameDescription, TimeRange};
use crate::enums::{
    AuthenticationType, Connector, Currency, FutureUsage, IntentStatus, PaymentMethod,
    PaymentMethodType,
};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct PaymentIntentFilters {
    #[serde(default)]
    pub status: Vec<IntentStatus>,
    #[serde(default)]
    pub currency: Vec<Currency>,
    #[serde(default)]
    pub profile_id: Vec<id_type::ProfileId>,
    #[serde(default)]
    pub connector: Vec<Connector>,
    #[serde(default)]
    pub auth_type: Vec<AuthenticationType>,
    #[serde(default)]
    pub payment_method: Vec<PaymentMethod>,
    #[serde(default)]
    pub payment_method_type: Vec<PaymentMethodType>,
    #[serde(default)]
    pub card_network: Vec<String>,
    #[serde(default)]
    pub merchant_id: Vec<id_type::MerchantId>,
    #[serde(default)]
    pub card_last_4: Vec<String>,
    #[serde(default)]
    pub card_issuer: Vec<String>,
    #[serde(default)]
    pub error_reason: Vec<String>,
    #[serde(default)]
    pub customer_id: Vec<id_type::CustomerId>,
    #[serde(default)]
    pub off_session: Vec<bool>,
    #[serde(default)]
    pub setup_future_usage: Vec<FutureUsage>,
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
pub enum PaymentIntentDimensions {
    #[strum(serialize = "status")]
    #[serde(rename = "status")]
    PaymentIntentStatus,
    Currency,
    ProfileId,
    Connector,
    #[strum(serialize = "authentication_type")]
    #[serde(rename = "authentication_type")]
    AuthType,
    PaymentMethod,
    PaymentMethodType,
    CardNetwork,
    MerchantId,
    #[strum(serialize = "card_last_4")]
    #[serde(rename = "card_last_4")]
    CardLast4,
    CardIssuer,
    ErrorReason,
    OffSession,
    SetupFutureUsage,
}

impl PaymentIntentDimensions {
    pub fn description(&self) -> &'static str {
        match self {
            Self::PaymentIntentStatus => "Status of the payment, such as succeeded or failed.",
            Self::Currency => "Currency of the payment.",
            Self::ProfileId => "Business profile.",
            Self::Connector => "Connector of the payment's attempt. Sessionized metrics only.",
            Self::AuthType => "three_ds or no_three_ds. Sessionized metrics only.",
            Self::PaymentMethod => "Payment method, such as card. Sessionized metrics only.",
            Self::PaymentMethodType => {
                "Payment method type, such as credit. Sessionized metrics only."
            }
            Self::CardNetwork => "Card network, such as Visa. Sessionized metrics only.",
            Self::MerchantId => "Merchant account.",
            Self::CardLast4 => "Last four card digits. Sessionized metrics only.",
            Self::CardIssuer => "Bank that issued the card. Sessionized metrics only.",
            Self::ErrorReason => "Error reason from the connector. Sessionized metrics only.",
            Self::OffSession => {
                "true when the merchant initiated the payment (MIT). Empty for customer payments."
            }
            Self::SetupFutureUsage => {
                "Whether the payment saves the card: off_session for future MITs, on_session for checkouts."
            }
        }
    }
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
pub enum PaymentIntentMetrics {
    SuccessfulSmartRetries,
    TotalSmartRetries,
    SmartRetriedAmount,
    PaymentIntentCount,
    PaymentsSuccessRate,
    PaymentProcessedAmount,
    SessionizedSuccessfulSmartRetries,
    SessionizedTotalSmartRetries,
    SessionizedSmartRetriedAmount,
    SessionizedPaymentIntentCount,
    SessionizedPaymentsSuccessRate,
    SessionizedPaymentProcessedAmount,
    SessionizedPaymentsDistribution,
}

impl PaymentIntentMetrics {
    pub fn description(&self) -> &'static str {
        match self {
            Self::SuccessfulSmartRetries => "Succeeded payments that needed more than one attempt.",
            Self::TotalSmartRetries => "Payments that needed more than one attempt.",
            Self::SmartRetriedAmount => "Amount of succeeded payments that needed a retry.",
            Self::PaymentIntentCount => "Payments created, all statuses.",
            Self::PaymentsSuccessRate => {
                "Succeeded payments ÷ payments, excluding those still awaiting the customer or merchant."
            }
            Self::PaymentProcessedAmount => "Amount of succeeded payments, per currency.",
            Self::SessionizedSuccessfulSmartRetries => {
                "successful_smart_retries, from the sessionizer."
            }
            Self::SessionizedTotalSmartRetries => "total_smart_retries, from the sessionizer.",
            Self::SessionizedSmartRetriedAmount => "smart_retried_amount, from the sessionizer.",
            Self::SessionizedPaymentIntentCount => "payment_intent_count, from the sessionizer.",
            Self::SessionizedPaymentsSuccessRate => "payments_success_rate, from the sessionizer.",
            Self::SessionizedPaymentProcessedAmount => {
                "payment_processed_amount, from the sessionizer."
            }
            Self::SessionizedPaymentsDistribution => {
                "Success and failure rates with and without smart retries, from the sessionizer."
            }
        }
    }
}

impl ForexMetric for PaymentIntentMetrics {
    fn is_forex_metric(&self) -> bool {
        matches!(
            self,
            Self::PaymentProcessedAmount
                | Self::SmartRetriedAmount
                | Self::SessionizedPaymentProcessedAmount
                | Self::SessionizedSmartRetriedAmount
        )
    }
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ErrorResult {
    pub reason: String,
    pub count: i64,
    pub percentage: f64,
}

pub mod metric_behaviour {
    pub struct SuccessfulSmartRetries;
    pub struct TotalSmartRetries;
    pub struct SmartRetriedAmount;
    pub struct PaymentIntentCount;
    pub struct PaymentsSuccessRate;
}

impl From<PaymentIntentMetrics> for NameDescription {
    fn from(value: PaymentIntentMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: value.description().to_string(),
        }
    }
}

impl From<PaymentIntentDimensions> for NameDescription {
    fn from(value: PaymentIntentDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: value.description().to_string(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct PaymentIntentMetricsBucketIdentifier {
    pub status: Option<IntentStatus>,
    pub currency: Option<Currency>,
    pub profile_id: Option<String>,
    pub connector: Option<String>,
    pub auth_type: Option<AuthenticationType>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub card_network: Option<String>,
    pub merchant_id: Option<String>,
    pub card_last_4: Option<String>,
    pub card_issuer: Option<String>,
    pub error_reason: Option<String>,
    pub off_session: Option<bool>,
    pub setup_future_usage: Option<FutureUsage>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl PaymentIntentMetricsBucketIdentifier {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        status: Option<IntentStatus>,
        currency: Option<Currency>,
        profile_id: Option<String>,
        connector: Option<String>,
        auth_type: Option<AuthenticationType>,
        payment_method: Option<String>,
        payment_method_type: Option<String>,
        card_network: Option<String>,
        merchant_id: Option<String>,
        card_last_4: Option<String>,
        card_issuer: Option<String>,
        error_reason: Option<String>,
        off_session: Option<bool>,
        setup_future_usage: Option<FutureUsage>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            status,
            currency,
            profile_id,
            connector,
            auth_type,
            payment_method,
            payment_method_type,
            card_network,
            merchant_id,
            card_last_4,
            card_issuer,
            error_reason,
            off_session,
            setup_future_usage,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

impl Hash for PaymentIntentMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.status.map(|i| i.to_string()).hash(state);
        self.currency.hash(state);
        self.profile_id.hash(state);
        self.connector.hash(state);
        self.auth_type.map(|i| i.to_string()).hash(state);
        self.payment_method.hash(state);
        self.payment_method_type.hash(state);
        self.card_network.hash(state);
        self.merchant_id.hash(state);
        self.card_last_4.hash(state);
        self.card_issuer.hash(state);
        self.error_reason.hash(state);
        self.off_session.hash(state);
        self.setup_future_usage.map(|i| i.to_string()).hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for PaymentIntentMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentIntentMetricsBucketValue {
    pub successful_smart_retries: Option<u64>,
    pub total_smart_retries: Option<u64>,
    pub smart_retried_amount: Option<u64>,
    pub smart_retried_amount_in_usd: Option<u64>,
    pub smart_retried_amount_without_smart_retries: Option<u64>,
    pub smart_retried_amount_without_smart_retries_in_usd: Option<u64>,
    pub payment_intent_count: Option<u64>,
    pub successful_payments: Option<u32>,
    pub successful_payments_without_smart_retries: Option<u32>,
    pub total_payments: Option<u32>,
    pub payments_success_rate: Option<f64>,
    pub payments_success_rate_without_smart_retries: Option<f64>,
    pub payment_processed_amount: Option<u64>,
    pub payment_processed_amount_in_usd: Option<u64>,
    pub payment_processed_count: Option<u64>,
    pub payment_processed_amount_without_smart_retries: Option<u64>,
    pub payment_processed_amount_without_smart_retries_in_usd: Option<u64>,
    pub payment_processed_count_without_smart_retries: Option<u64>,
    pub payments_success_rate_distribution_without_smart_retries: Option<f64>,
    pub payments_failure_rate_distribution_without_smart_retries: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: PaymentIntentMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: PaymentIntentMetricsBucketIdentifier,
}
