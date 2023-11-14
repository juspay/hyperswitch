use std::collections::HashSet;

use common_utils::events::ApiEventMetric;
use time::PrimitiveDateTime;

use self::{
    payments::{PaymentDimensions, PaymentMetrics},
    refunds::{RefundDimensions, RefundMetrics},
};

pub mod payments;
pub mod refunds;

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

impl ApiEventMetric for GetInfoResponse {}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub start_time: PrimitiveDateTime,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub end_time: Option<PrimitiveDateTime>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, masking::Serialize)]
pub struct TimeSeries {
    pub granularity: Granularity,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, masking::Serialize)]
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

#[derive(Clone, Debug, serde::Deserialize, masking::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentMetricRequest {
    pub time_series: Option<TimeSeries>,
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<PaymentDimensions>,
    #[serde(default)]
    pub filters: payments::PaymentFilters,
    pub metrics: HashSet<PaymentMetrics>,
    #[serde(default)]
    pub delta: bool,
}

impl ApiEventMetric for GetPaymentMetricRequest {}

#[derive(Clone, Debug, serde::Deserialize, masking::Serialize)]
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

impl ApiEventMetric for GetRefundMetricRequest {}

#[derive(Debug, serde::Serialize)]
pub struct AnalyticsMetadata {
    pub current_time_range: TimeRange,
}

#[derive(Debug, serde::Deserialize, masking::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPaymentFiltersRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<PaymentDimensions>,
}

impl ApiEventMetric for GetPaymentFiltersRequest {}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFiltersResponse {
    pub query_data: Vec<FilterValue>,
}

impl ApiEventMetric for PaymentFiltersResponse {}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterValue {
    pub dimension: PaymentDimensions,
    pub values: Vec<String>,
}

#[derive(Debug, serde::Deserialize, masking::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRefundFilterRequest {
    pub time_range: TimeRange,
    #[serde(default)]
    pub group_by_names: Vec<RefundDimensions>,
}

impl ApiEventMetric for GetRefundFilterRequest {}

#[derive(Debug, Default, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundFiltersResponse {
    pub query_data: Vec<RefundFilterValue>,
}

impl ApiEventMetric for RefundFiltersResponse {}

#[derive(Debug, serde::Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefundFilterValue {
    pub dimension: RefundDimensions,
    pub values: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsResponse<T> {
    pub query_data: Vec<T>,
    pub meta_data: [AnalyticsMetadata; 1],
}
