use api_models::analytics::{
    refunds::{
        RefundDimensions, RefundFilters, RefundMetrics, RefundMetricsBucketIdentifier, RefundType,
    },
    Granularity, TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;
mod refund_count;
mod refund_processed_amount;
mod refund_success_count;
mod refund_success_rate;
use refund_count::RefundCount;
use refund_processed_amount::RefundProcessedAmount;
use refund_success_count::RefundSuccessCount;
use refund_success_rate::RefundSuccessRate;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};
#[derive(Debug, Eq, PartialEq, serde::Deserialize)]
pub struct RefundMetricRow {
    pub currency: Option<DBEnumWrapper<storage_enums::Currency>>,
    pub refund_status: Option<DBEnumWrapper<storage_enums::RefundStatus>>,
    pub connector: Option<String>,
    pub refund_type: Option<DBEnumWrapper<RefundType>>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait RefundMetricAnalytics: LoadRow<RefundMetricRow> {}

#[async_trait::async_trait]
pub trait RefundMetric<T>
where
    T: AnalyticsDataSource + RefundMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[RefundDimensions],
        merchant_id: &str,
        filters: &RefundFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(RefundMetricsBucketIdentifier, RefundMetricRow)>>;
}

#[async_trait::async_trait]
impl<T> RefundMetric<T> for RefundMetrics
where
    T: AnalyticsDataSource + RefundMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[RefundDimensions],
        merchant_id: &str,
        filters: &RefundFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(RefundMetricsBucketIdentifier, RefundMetricRow)>> {
        match self {
            Self::RefundSuccessRate => {
                RefundSuccessRate::default()
                    .load_metrics(
                        dimensions,
                        merchant_id,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::RefundCount => {
                RefundCount::default()
                    .load_metrics(
                        dimensions,
                        merchant_id,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::RefundSuccessCount => {
                RefundSuccessCount::default()
                    .load_metrics(
                        dimensions,
                        merchant_id,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::RefundProcessedAmount => {
                RefundProcessedAmount::default()
                    .load_metrics(
                        dimensions,
                        merchant_id,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
        }
    }
}
