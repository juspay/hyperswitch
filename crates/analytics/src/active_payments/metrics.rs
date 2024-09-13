use std::collections::HashSet;

use api_models::analytics::{
    active_payments::{ActivePaymentsMetrics, ActivePaymentsMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, LoadRow, MetricsResult},
};

mod active_payments;

use active_payments::ActivePayments;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, Hash)]
pub struct ActivePaymentsMetricRow {
    pub count: Option<i64>,
}

pub trait ActivePaymentsMetricAnalytics: LoadRow<ActivePaymentsMetricRow> {}

#[async_trait::async_trait]
pub trait ActivePaymentsMetric<T>
where
    T: AnalyticsDataSource + ActivePaymentsMetricAnalytics,
{
    async fn load_metrics(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        publishable_key: &str,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<
        HashSet<(
            ActivePaymentsMetricsBucketIdentifier,
            ActivePaymentsMetricRow,
        )>,
    >;
}

#[async_trait::async_trait]
impl<T> ActivePaymentsMetric<T> for ActivePaymentsMetrics
where
    T: AnalyticsDataSource + ActivePaymentsMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        publishable_key: &str,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<
        HashSet<(
            ActivePaymentsMetricsBucketIdentifier,
            ActivePaymentsMetricRow,
        )>,
    > {
        match self {
            Self::ActivePayments => {
                ActivePayments
                    .load_metrics(merchant_id, publishable_key, time_range, pool)
                    .await
            }
        }
    }
}
