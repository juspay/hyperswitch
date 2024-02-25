mod dispute_status_metric;
mod total_amount_disputed;
mod total_dispute_lost_amount;

use api_models::{
    analytics::{
        disputes::{
            DisputeDimensions, DisputeFilters, DisputeMetrics, DisputeMetricsBucketIdentifier,
        },
        Granularity,
    },
    payments::TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;

use self::{
    dispute_status_metric::DisputeStatusMetric, total_amount_disputed::TotalAmountDisputed,
    total_dispute_lost_amount::TotalDisputeLostAmount,
};
use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};
#[derive(Debug, Eq, PartialEq, serde::Deserialize)]
pub struct DisputeMetricRow {
    pub dispute_stage: Option<DBEnumWrapper<storage_enums::DisputeStage>>,
    pub dispute_status: Option<DBEnumWrapper<storage_enums::DisputeStatus>>,
    pub connector: Option<String>,
    pub connector_status: Option<String>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait DisputeMetricAnalytics: LoadRow<DisputeMetricRow> {}

#[async_trait::async_trait]
pub trait DisputeMetric<T>
where
    T: AnalyticsDataSource + DisputeMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[DisputeDimensions],
        merchant_id: &str,
        filters: &DisputeFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(DisputeMetricsBucketIdentifier, DisputeMetricRow)>>;
}

#[async_trait::async_trait]
impl<T> DisputeMetric<T> for DisputeMetrics
where
    T: AnalyticsDataSource + DisputeMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[DisputeDimensions],
        merchant_id: &str,
        filters: &DisputeFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(DisputeMetricsBucketIdentifier, DisputeMetricRow)>> {
        match self {
            Self::TotalAmountDisputed => {
                TotalAmountDisputed::default()
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
            Self::DisputeStatusMetric => {
                DisputeStatusMetric::default()
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
            Self::TotalDisputeLostAmount => {
                TotalDisputeLostAmount::default()
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
