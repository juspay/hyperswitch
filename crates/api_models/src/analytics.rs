use std::collections::HashSet;

pub use common_utils::types::TimeRange;
use common_utils::{events::ApiEventMetric, pii::EmailStrategy, types::authentication::AuthInfo};
use masking::Secret;

use self::{
    active_payments::ActivePaymentsMetrics,
    api_event::{ApiEventDimensions, ApiEventMetrics},
    auth_events::{AuthEventDimensions, AuthEventFilters, AuthEventMetrics},
    disputes::{DisputeDimensions, DisputeMetrics},
    frm::{FrmDimensions, FrmMetrics},
    payment_intents::{PaymentIntentDimensions, PaymentIntentMetrics},
    payments::{PaymentDimensions, PaymentDistributions, PaymentMetrics},
    refunds::{RefundDimensions, RefundDistributions, RefundMetrics},
    sdk_events::{SdkEventDimensions, SdkEventMetrics},
};
pub mod active_payments;
pub mod api_event;
pub mod auth_events;
pub mod connector_events;
pub mod disputes;
pub mod frm;
pub mod outgoing_webhook_event;
pub mod payment_intents;
pub mod payments;
pub mod refunds;
pub mod sdk_events;
pub mod search;

#[derive(Debug, serde::Serialize)]
pub struct NameDescription {
    pub name: String,
    pub desc: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInfoResponse {
    pub metrics: Vec<NameDescription>,
    pub download_dimensions: Option<Vec<NameDescription>>,
    pub dimensions: Vec<NameDescription>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub struct TimeSeries {
    pub granularity: Granularity,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum Granularity {
    #[serde(rename = "G_ONEMIN")]
    OneMin,
    #[serde(rename = "G_FIVEMIN")]
    FiveMin,
    #[serde(rename = "G_FIFTEENMIN")]
    FifteenMin,
    #[serde(rename = "G_THIRTYMIN")]
    ThirtyMin,
    #[serde(rename = "G_ONEHOUR")]
    OneHour,
    #[serde(rename = "G_ONEDAY")]
    OneDay,
}
pub trait ForexMetric {
    fn is_forex_metric(&self) -> bool;
}

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsRequest {
    pub payment_intent: Option<GetPaymentIntentMetricRequest>,
    pub payment_attempt: Option<GetPaymentMetricRequest>,
    pub refund: Option<GetRefundMetricRequest>,
    pub dispute: Option<GetDisputeMetricRequest>,
}

impl AnalyticsRequest {
    pub fn requires_forex_functionality(&self) -> bool {
        self.payment_attempt
            .as_ref()
            .map(|req| req.metrics.iter().any(|metric| metric.is_forex_metric()))
            .unwrap_or_default()
            || self
                .payment_intent
                .as_ref()
                .map(|req| req.metrics.iter().any(|metric| metric.is_forex_metric()))
                .unwrap_or_default()
            || self
                .refund
                .as_ref()
                .map(|req| req.metrics.iter().any(|metric| metric.is_forex_metric()))
                .unwrap_or_default()
            || self
                .dispute
                .as_ref()
                .map(|req| req.metrics.iter().any(|metric| metric.is_forex_metric()))
                .unwrap_or_default()
    }
}
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<PaymentDimensions>,
    #[serde(default)]
    pub filters: payments::PaymentFilters,
    pub metrics: HashSet<PaymentMetrics>,
    pub distribution: Option<PaymentDistributionBody>,
    #[serde(default)]
    pub delta: bool,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum QueryLimit {
    #[serde(rename = "TOP_5")]
    Top5,
    #[serde(rename = "TOP_10")]
    Top10,
}

#[allow(clippy::from_over_into)]
impl Into<u64> for QueryLimit {
    fn into(self) -> u64 {
        match self {
            Self::Top5 => 5,
            Self::Top10 => 10,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDistributionBody {
    pub distribution_for: PaymentDistributions,
    pub distribution_cardinality: QueryLimit,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundDistributionBody {
    pub distribution_for: RefundDistributions,
    pub distribution_cardinality: QueryLimit,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRequest {
    pub time_range: TimeRange,
    pub emails: Option<Vec<Secret<String, EmailStrategy>>>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReportRequest {
    pub request: ReportRequest,
    pub merchant_id: Option<common_utils::id_type::MerchantId>,
    pub auth: AuthInfo,
    pub email: Secret<String, EmailStrategy>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentIntentMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<PaymentIntentDimensions>,
    #[serde(default)]
    pub filters: payment_intents::PaymentIntentFilters,
    pub metrics: HashSet<PaymentIntentMetrics>,
    #[serde(default)]
    pub delta: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRefundMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<RefundDimensions>,
    #[serde(default)]
    pub filters: refunds::RefundFilters,
    pub metrics: HashSet<RefundMetrics>,
    pub distribution: Option<RefundDistributionBody>,
    #[serde(default)]
    pub delta: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrmMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<FrmDimensions>,
    #[serde(default)]
    pub filters: frm::FrmFilters,
    pub metrics: HashSet<FrmMetrics>,
    #[serde(default)]
    pub delta: bool,
}

impl ApiEventMetric for GetFrmMetricRequest {}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSdkEventMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<SdkEventDimensions>,
    #[serde(default)]
    pub filters: sdk_events::SdkEventFilters,
    pub metrics: HashSet<SdkEventMetrics>,
    #[serde(default)]
    pub delta: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAuthEventMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<AuthEventDimensions>,
    #[serde(default)]
    pub filters: AuthEventFilters,
    #[serde(default)]
    pub metrics: HashSet<AuthEventMetrics>,
    #[serde(default)]
    pub delta: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetActivePaymentsMetricRequest {
    #[serde(default)]
    pub metrics: HashSet<ActivePaymentsMetrics>,
    pub time_range: TimeRange,
}

#[derive(Debug, serde::Serialize)]
pub struct AnalyticsMetadata {
    pub current_time_range: TimeRange,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentsAnalyticsMetadata {
    pub total_payment_processed_amount: Option<u64>,
    pub total_payment_processed_amount_in_usd: Option<u64>,
    pub total_payment_processed_amount_without_smart_retries: Option<u64>,
    pub total_payment_processed_amount_without_smart_retries_usd: Option<u64>,
    pub total_payment_processed_count: Option<u64>,
    pub total_payment_processed_count_without_smart_retries: Option<u64>,
    pub total_failure_reasons_count: Option<u64>,
    pub total_failure_reasons_count_without_smart_retries: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentIntentsAnalyticsMetadata {
    pub total_success_rate: Option<f64>,
    pub total_success_rate_without_smart_retries: Option<f64>,
    pub total_smart_retried_amount: Option<u64>,
    pub total_smart_retried_amount_without_smart_retries: Option<u64>,
    pub total_payment_processed_amount: Option<u64>,
    pub total_payment_processed_amount_without_smart_retries: Option<u64>,
    pub total_smart_retried_amount_in_usd: Option<u64>,
    pub total_smart_retried_amount_without_smart_retries_in_usd: Option<u64>,
    pub total_payment_processed_amount_in_usd: Option<u64>,
    pub total_payment_processed_amount_without_smart_retries_in_usd: Option<u64>,
    pub total_payment_processed_count: Option<u64>,
    pub total_payment_processed_count_without_smart_retries: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct RefundsAnalyticsMetadata {
    pub total_refund_success_rate: Option<f64>,
    pub total_refund_processed_amount: Option<u64>,
    pub total_refund_processed_amount_in_usd: Option<u64>,
    pub total_refund_processed_count: Option<u64>,
    pub total_refund_reason_count: Option<u64>,
    pub total_refund_error_message_count: Option<u64>,
}
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentFiltersRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<PaymentDimensions>,
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFiltersResponse {
    pub query_data: Vec<FilterValue>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterValue {
    pub dimension: PaymentDimensions,
    pub values: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentIntentFiltersRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<PaymentIntentDimensions>,
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentIntentFiltersResponse {
    pub query_data: Vec<PaymentIntentFilterValue>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentIntentFilterValue {
    pub dimension: PaymentIntentDimensions,
    pub values: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRefundFilterRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<RefundDimensions>,
}

#[derive(Debug, Default, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundFiltersResponse {
    pub query_data: Vec<RefundFilterValue>,
}

#[derive(Debug, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundFilterValue {
    pub dimension: RefundDimensions,
    pub values: Vec<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrmFilterRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<FrmDimensions>,
}

impl ApiEventMetric for GetFrmFilterRequest {}

#[derive(Debug, Default, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrmFiltersResponse {
    pub query_data: Vec<FrmFilterValue>,
}

impl ApiEventMetric for FrmFiltersResponse {}

#[derive(Debug, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrmFilterValue {
    pub dimension: FrmDimensions,
    pub values: Vec<String>,
}

impl ApiEventMetric for FrmFilterValue {}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSdkEventFiltersRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<SdkEventDimensions>,
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkEventFiltersResponse {
    pub query_data: Vec<SdkEventFilterValue>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkEventFilterValue {
    pub dimension: SdkEventDimensions,
    pub values: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct DisputesAnalyticsMetadata {
    pub total_disputed_amount: Option<u64>,
    pub total_dispute_lost_amount: Option<u64>,
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [AnalyticsMetadata; 1],
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentsMetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [PaymentsAnalyticsMetadata; 1],
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentIntentsMetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [PaymentIntentsAnalyticsMetadata; 1],
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundsMetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [RefundsAnalyticsMetadata; 1],
}
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisputesMetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [DisputesAnalyticsMetadata; 1],
}
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiEventFiltersRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<ApiEventDimensions>,
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiEventFiltersResponse {
    pub query_data: Vec<ApiEventFilterValue>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiEventFilterValue {
    pub dimension: ApiEventDimensions,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetApiEventMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<ApiEventDimensions>,
    #[serde(default)]
    pub filters: api_event::ApiEventFilters,
    pub metrics: HashSet<ApiEventMetrics>,
    #[serde(default)]
    pub delta: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDisputeFilterRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<DisputeDimensions>,
}

#[derive(Debug, Default, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DisputeFiltersResponse {
    pub query_data: Vec<DisputeFilterValue>,
}

#[derive(Debug, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DisputeFilterValue {
    pub dimension: DisputeDimensions,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDisputeMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<DisputeDimensions>,
    #[serde(default)]
    pub filters: disputes::DisputeFilters,
    pub metrics: HashSet<DisputeMetrics>,
    #[serde(default)]
    pub delta: bool,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SankeyResponse {
    pub count: i64,
    pub status: String,
    pub refunds_status: Option<String>,
    pub dispute_status: Option<String>,
    pub first_attempt: i64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAuthEventFilterRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<AuthEventDimensions>,
}

#[derive(Debug, Default, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthEventFiltersResponse {
    pub query_data: Vec<AuthEventFilterValue>,
}

#[derive(Debug, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthEventFilterValue {
    pub dimension: AuthEventDimensions,
    pub values: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthEventMetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [AuthEventsAnalyticsMetadata; 1],
}

#[derive(Debug, serde::Serialize)]
pub struct AuthEventsAnalyticsMetadata {
    pub total_error_message_count: Option<u64>,
}
