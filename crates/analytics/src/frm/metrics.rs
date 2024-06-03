use api_models::analytics::{
    frm::{
        FrmDimensions, FrmFilters, FrmMetrics, FrmMetricsBucketIdentifier, FrmType,
    },
    Granularity, TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;
// mod refund_count;
// mod refund_processed_amount;
// mod refund_success_count;
// mod refund_success_rate;
mod frm_triggered_attempts;
mod frm_blocked_rate;
// use refund_count::RefundCount;
// use refund_processed_amount::RefundProcessedAmount;
// use refund_success_count::RefundSuccessCount;
// use refund_success_rate::RefundSuccessRate;
use frm_triggered_attempts::FrmTriggeredAttempts;
use frm_blocked_rate::FrmBlockedRate;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};
#[derive(Debug, Eq, PartialEq, serde::Deserialize)]
// TODO (tsdk02) - Should implement this
pub struct FrmMetricRow {
    pub frm_name: Option<String>,
    pub frm_status: Option<DBEnumWrapper<storage_enums::FrmStatus>>,
    pub frm_transaction_type: Option<DBEnumWrapper<storage_enums::FrmType>>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait FrmMetricAnalytics: LoadRow<FrmMetricRow> {}

#[async_trait::async_trait]
pub trait FrmMetric<T>
where
    T: AnalyticsDataSource + FrmMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[FrmDimensions],
        merchant_id: &str,
        filters: &FrmFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(FrmMetricsBucketIdentifier, FrmMetricRow)>>;
}

#[async_trait::async_trait]
impl<T> FrmMetric<T> for FrmMetrics
where
    T: AnalyticsDataSource + FrmMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[FrmDimensions],
        merchant_id: &str,
        filters: &FrmFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(FrmMetricsBucketIdentifier, FrmMetricRow)>> {
        match self {
            Self::FrmSuccessRate => {
                FrmSuccessRate::default()
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
