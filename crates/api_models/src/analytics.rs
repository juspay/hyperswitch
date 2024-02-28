use std::collections::HashSet;

use common_utils::pii::EmailStrategy;
use masking::Secret;

use self::{
    api_event::{ApiEventDimensions, ApiEventMetrics},
    disputes::DisputeDimensions,
    payments::{PaymentDimensions, PaymentDistributions, PaymentMetrics},
    refunds::{RefundDimensions, RefundMetrics},
    sdk_events::{SdkEventDimensions, SdkEventMetrics},
};
pub use crate::payments::TimeRange;

pub mod api_event;
pub mod connector_events;
pub mod disputes;
pub mod outgoing_webhook_event;
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
    pub distribution: Option<Distribution>,
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
pub struct Distribution {
    pub distribution_for: PaymentDistributions,
    pub distribution_cardinality: QueryLimit,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRequest {
    pub time_range: TimeRange,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReportRequest {
    pub request: ReportRequest,
    pub merchant_id: String,
    pub email: Secret<String, EmailStrategy>,
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
    #[serde(default)]
    pub delta: bool,
}

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

#[derive(Debug, serde::Serialize)]
pub struct AnalyticsMetadata {
    pub current_time_range: TimeRange,
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
#[serde(rename_all = "camelCase")]
pub struct MetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [AnalyticsMetadata; 1],
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
