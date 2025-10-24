use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use common_utils::id_type;

use super::{ForexMetric, NameDescription, TimeRange};
use crate::enums::{
    AttemptStatus, AuthenticationType, CardNetwork, Connector, Currency, PaymentMethod,
    PaymentMethodType, RoutingApproach,
};

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct PaymentFilters {
    #[serde(default)]
    pub currency: Vec<Currency>,
    #[serde(default)]
    pub status: Vec<AttemptStatus>,
    #[serde(default)]
    pub connector: Vec<Connector>,
    #[serde(default)]
    pub auth_type: Vec<AuthenticationType>,
    #[serde(default)]
    pub payment_method: Vec<PaymentMethod>,
    #[serde(default)]
    pub payment_method_type: Vec<PaymentMethodType>,
    #[serde(default)]
    pub client_source: Vec<String>,
    #[serde(default)]
    pub client_version: Vec<String>,
    #[serde(default)]
    pub card_network: Vec<CardNetwork>,
    #[serde(default)]
    pub profile_id: Vec<id_type::ProfileId>,
    #[serde(default)]
    pub merchant_id: Vec<id_type::MerchantId>,
    #[serde(default)]
    pub card_last_4: Vec<String>,
    #[serde(default)]
    pub card_issuer: Vec<String>,
    #[serde(default)]
    pub error_reason: Vec<String>,
    #[serde(default)]
    pub first_attempt: Vec<bool>,
    #[serde(default)]
    pub routing_approach: Vec<RoutingApproach>,
    #[serde(default)]
    pub signature_network: Vec<String>,
    #[serde(default)]
    pub is_issuer_regulated: Vec<bool>,
    #[serde(default)]
    pub is_debit_routed: Vec<bool>,
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
pub enum PaymentDimensions {
    // Do not change the order of these enums
    // Consult the Dashboard FE folks since these also affects the order of metrics on FE
    Connector,
    PaymentMethod,
    PaymentMethodType,
    Currency,
    #[strum(serialize = "authentication_type")]
    #[serde(rename = "authentication_type")]
    AuthType,
    #[strum(serialize = "status")]
    #[serde(rename = "status")]
    PaymentStatus,
    ClientSource,
    ClientVersion,
    ProfileId,
    CardNetwork,
    MerchantId,
    #[strum(serialize = "card_last_4")]
    #[serde(rename = "card_last_4")]
    CardLast4,
    CardIssuer,
    ErrorReason,
    RoutingApproach,
    SignatureNetwork,
    IsIssuerRegulated,
    IsDebitRouted,
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
pub enum PaymentMetrics {
    PaymentSuccessRate,
    PaymentCount,
    PaymentSuccessCount,
    PaymentProcessedAmount,
    AvgTicketSize,
    RetriesCount,
    ConnectorSuccessRate,
    DebitRouting,
    SessionizedPaymentSuccessRate,
    SessionizedPaymentCount,
    SessionizedPaymentSuccessCount,
    SessionizedPaymentProcessedAmount,
    SessionizedAvgTicketSize,
    SessionizedRetriesCount,
    SessionizedConnectorSuccessRate,
    SessionizedDebitRouting,
    PaymentsDistribution,
    FailureReasons,
}

impl ForexMetric for PaymentMetrics {
    fn is_forex_metric(&self) -> bool {
        matches!(
            self,
            Self::PaymentProcessedAmount
                | Self::AvgTicketSize
                | Self::DebitRouting
                | Self::SessionizedPaymentProcessedAmount
                | Self::SessionizedAvgTicketSize
                | Self::SessionizedDebitRouting,
        )
    }
}
#[derive(Debug, Default, serde::Serialize)]
pub struct ErrorResult {
    pub reason: String,
    pub count: i64,
    pub percentage: f64,
}

#[derive(
    Clone,
    Copy,
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
pub enum PaymentDistributions {
    #[strum(serialize = "error_message")]
    PaymentErrorMessage,
}

pub mod metric_behaviour {
    pub struct PaymentSuccessRate;
    pub struct PaymentCount;
    pub struct PaymentSuccessCount;
    pub struct PaymentProcessedAmount;
    pub struct AvgTicketSize;
}

impl From<PaymentMetrics> for NameDescription {
    fn from(value: PaymentMetrics) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

impl From<PaymentDimensions> for NameDescription {
    fn from(value: PaymentDimensions) -> Self {
        Self {
            name: value.to_string(),
            desc: String::new(),
        }
    }
}

#[derive(Debug, serde::Serialize, Eq)]
pub struct PaymentMetricsBucketIdentifier {
    pub currency: Option<Currency>,
    pub status: Option<AttemptStatus>,
    pub connector: Option<String>,
    #[serde(rename = "authentication_type")]
    pub auth_type: Option<AuthenticationType>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub client_source: Option<String>,
    pub client_version: Option<String>,
    pub profile_id: Option<String>,
    pub card_network: Option<String>,
    pub merchant_id: Option<String>,
    pub card_last_4: Option<String>,
    pub card_issuer: Option<String>,
    pub error_reason: Option<String>,
    pub routing_approach: Option<RoutingApproach>,
    pub signature_network: Option<String>,
    pub is_issuer_regulated: Option<bool>,
    pub is_debit_routed: Option<bool>,
    #[serde(rename = "time_range")]
    pub time_bucket: TimeRange,
    // Coz FE sucks
    #[serde(rename = "time_bucket")]
    #[serde(with = "common_utils::custom_serde::iso8601custom")]
    pub start_time: time::PrimitiveDateTime,
}

impl PaymentMetricsBucketIdentifier {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        currency: Option<Currency>,
        status: Option<AttemptStatus>,
        connector: Option<String>,
        auth_type: Option<AuthenticationType>,
        payment_method: Option<String>,
        payment_method_type: Option<String>,
        client_source: Option<String>,
        client_version: Option<String>,
        profile_id: Option<String>,
        card_network: Option<String>,
        merchant_id: Option<String>,
        card_last_4: Option<String>,
        card_issuer: Option<String>,
        error_reason: Option<String>,
        routing_approach: Option<RoutingApproach>,
        signature_network: Option<String>,
        is_issuer_regulated: Option<bool>,
        is_debit_routed: Option<bool>,
        normalized_time_range: TimeRange,
    ) -> Self {
        Self {
            currency,
            status,
            connector,
            auth_type,
            payment_method,
            payment_method_type,
            client_source,
            client_version,
            profile_id,
            card_network,
            merchant_id,
            card_last_4,
            card_issuer,
            error_reason,
            routing_approach,
            signature_network,
            is_issuer_regulated,
            is_debit_routed,
            time_bucket: normalized_time_range,
            start_time: normalized_time_range.start_time,
        }
    }
}

impl Hash for PaymentMetricsBucketIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.currency.hash(state);
        self.status.map(|i| i.to_string()).hash(state);
        self.connector.hash(state);
        self.auth_type.map(|i| i.to_string()).hash(state);
        self.payment_method.hash(state);
        self.payment_method_type.hash(state);
        self.client_source.hash(state);
        self.client_version.hash(state);
        self.profile_id.hash(state);
        self.card_network.hash(state);
        self.merchant_id.hash(state);
        self.card_last_4.hash(state);
        self.card_issuer.hash(state);
        self.error_reason.hash(state);
        self.routing_approach
            .clone()
            .map(|i| i.to_string())
            .hash(state);
        self.signature_network.hash(state);
        self.is_issuer_regulated.hash(state);
        self.is_debit_routed.hash(state);
        self.time_bucket.hash(state);
    }
}

impl PartialEq for PaymentMetricsBucketIdentifier {
    fn eq(&self, other: &Self) -> bool {
        let mut left = DefaultHasher::new();
        self.hash(&mut left);
        let mut right = DefaultHasher::new();
        other.hash(&mut right);
        left.finish() == right.finish()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentMetricsBucketValue {
    pub payment_success_rate: Option<f64>,
    pub payment_count: Option<u64>,
    pub payment_success_count: Option<u64>,
    pub payment_processed_amount: Option<u64>,
    pub payment_processed_amount_in_usd: Option<u64>,
    pub payment_processed_count: Option<u64>,
    pub payment_processed_amount_without_smart_retries: Option<u64>,
    pub payment_processed_amount_without_smart_retries_usd: Option<u64>,
    pub payment_processed_count_without_smart_retries: Option<u64>,
    pub avg_ticket_size: Option<f64>,
    pub payment_error_message: Option<Vec<ErrorResult>>,
    pub retries_count: Option<u64>,
    pub retries_amount_processed: Option<u64>,
    pub connector_success_rate: Option<f64>,
    pub payments_success_rate_distribution: Option<f64>,
    pub payments_success_rate_distribution_without_smart_retries: Option<f64>,
    pub payments_success_rate_distribution_with_only_retries: Option<f64>,
    pub payments_failure_rate_distribution: Option<f64>,
    pub payments_failure_rate_distribution_without_smart_retries: Option<f64>,
    pub payments_failure_rate_distribution_with_only_retries: Option<f64>,
    pub failure_reason_count: Option<u64>,
    pub failure_reason_count_without_smart_retries: Option<u64>,
    pub debit_routed_transaction_count: Option<u64>,
    pub debit_routing_savings: Option<u64>,
    pub debit_routing_savings_in_usd: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct MetricsBucketResponse {
    #[serde(flatten)]
    pub values: PaymentMetricsBucketValue,
    #[serde(flatten)]
    pub dimensions: PaymentMetricsBucketIdentifier,
}
