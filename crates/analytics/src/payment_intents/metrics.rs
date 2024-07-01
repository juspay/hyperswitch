use api_models::analytics::{
    payment_intents::{
        PaymentIntentDimensions, PaymentIntentFilters, PaymentIntentMetrics,
        PaymentIntentMetricsBucketIdentifier,
    },
    Granularity, TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};

mod payment_intent_count;
mod smart_retried_amount;
mod successful_smart_retries;
mod total_smart_retries;

use payment_intent_count::PaymentIntentCount;
use smart_retried_amount::SmartRetriedAmount;
use successful_smart_retries::SuccessfulSmartRetries;
use total_smart_retries::TotalSmartRetries;

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct PaymentIntentMetricRow {
    pub status: Option<DBEnumWrapper<storage_enums::IntentStatus>>,
    pub currency: Option<DBEnumWrapper<storage_enums::Currency>>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait PaymentIntentMetricAnalytics: LoadRow<PaymentIntentMetricRow> {}

#[async_trait::async_trait]
pub trait PaymentIntentMetric<T>
where
    T: AnalyticsDataSource + PaymentIntentMetricAnalytics,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentIntentDimensions],
        merchant_id: &str,
        filters: &PaymentIntentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>>;
}

#[async_trait::async_trait]
impl<T> PaymentIntentMetric<T> for PaymentIntentMetrics
where
    T: AnalyticsDataSource + PaymentIntentMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentIntentDimensions],
        merchant_id: &str,
        filters: &PaymentIntentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>> {
        match self {
            Self::SuccessfulSmartRetries => {
                SuccessfulSmartRetries
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
            Self::TotalSmartRetries => {
                TotalSmartRetries
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
            Self::SmartRetriedAmount => {
                SmartRetriedAmount
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
            Self::PaymentIntentCount => {
                PaymentIntentCount
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
