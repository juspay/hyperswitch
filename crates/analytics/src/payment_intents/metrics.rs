use std::collections::HashSet;

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
    enums::AuthInfo,
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};

mod payment_intent_count;
mod payment_processed_amount;
mod payments_success_rate;
mod sessionized_metrics;
mod smart_retried_amount;
mod successful_smart_retries;
mod total_smart_retries;

use payment_intent_count::PaymentIntentCount;
use payment_processed_amount::PaymentProcessedAmount;
use payments_success_rate::PaymentsSuccessRate;
use smart_retried_amount::SmartRetriedAmount;
use successful_smart_retries::SuccessfulSmartRetries;
use total_smart_retries::TotalSmartRetries;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, Hash)]
pub struct PaymentIntentMetricRow {
    pub status: Option<DBEnumWrapper<storage_enums::IntentStatus>>,
    pub currency: Option<DBEnumWrapper<storage_enums::Currency>>,
    pub profile_id: Option<String>,
    pub connector: Option<String>,
    pub authentication_type: Option<DBEnumWrapper<storage_enums::AuthenticationType>>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub card_network: Option<String>,
    pub merchant_id: Option<String>,
    pub card_last_4: Option<String>,
    pub card_issuer: Option<String>,
    pub error_reason: Option<String>,
    pub first_attempt: Option<i64>,
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
        auth: &AuthInfo,
        filters: &PaymentIntentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>>;
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
        auth: &AuthInfo,
        filters: &PaymentIntentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>>
    {
        match self {
            Self::SuccessfulSmartRetries => {
                SuccessfulSmartRetries
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::TotalSmartRetries => {
                TotalSmartRetries
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SmartRetriedAmount => {
                SmartRetriedAmount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::PaymentIntentCount => {
                PaymentIntentCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::PaymentsSuccessRate => {
                PaymentsSuccessRate
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::PaymentProcessedAmount => {
                PaymentProcessedAmount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedSuccessfulSmartRetries => {
                sessionized_metrics::SuccessfulSmartRetries
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedTotalSmartRetries => {
                sessionized_metrics::TotalSmartRetries
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedSmartRetriedAmount => {
                sessionized_metrics::SmartRetriedAmount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentIntentCount => {
                sessionized_metrics::PaymentIntentCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentsSuccessRate => {
                sessionized_metrics::PaymentsSuccessRate
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentProcessedAmount => {
                sessionized_metrics::PaymentProcessedAmount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentsDistribution => {
                sessionized_metrics::PaymentsDistribution
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
        }
    }
}
