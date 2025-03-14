use std::collections::HashSet;

use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetrics, PaymentMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;

use crate::{
    enums::AuthInfo,
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};

mod avg_ticket_size;
mod connector_success_rate;
mod payment_count;
mod payment_processed_amount;
mod payment_success_count;
mod retries_count;
mod sessionized_metrics;
mod success_rate;

use avg_ticket_size::AvgTicketSize;
use connector_success_rate::ConnectorSuccessRate;
use payment_count::PaymentCount;
use payment_processed_amount::PaymentProcessedAmount;
use payment_success_count::PaymentSuccessCount;
use success_rate::PaymentSuccessRate;

use self::retries_count::RetriesCount;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, Hash)]
pub struct PaymentMetricRow {
    pub currency: Option<DBEnumWrapper<storage_enums::Currency>>,
    pub status: Option<DBEnumWrapper<storage_enums::AttemptStatus>>,
    pub connector: Option<String>,
    pub authentication_type: Option<DBEnumWrapper<storage_enums::AuthenticationType>>,
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
    pub first_attempt: Option<bool>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait PaymentMetricAnalytics: LoadRow<PaymentMetricRow> {}

#[async_trait::async_trait]
pub trait PaymentMetric<T>
where
    T: AnalyticsDataSource + PaymentMetricAnalytics,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentDimensions],
        auth: &AuthInfo,
        filters: &PaymentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>>;
}

#[async_trait::async_trait]
impl<T> PaymentMetric<T> for PaymentMetrics
where
    T: AnalyticsDataSource + PaymentMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentDimensions],
        auth: &AuthInfo,
        filters: &PaymentFilters,
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>> {
        match self {
            Self::PaymentSuccessRate => {
                PaymentSuccessRate
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::PaymentCount => {
                PaymentCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::PaymentSuccessCount => {
                PaymentSuccessCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::PaymentProcessedAmount => {
                PaymentProcessedAmount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::AvgTicketSize => {
                AvgTicketSize
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::RetriesCount => {
                RetriesCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::ConnectorSuccessRate => {
                ConnectorSuccessRate
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentSuccessRate => {
                sessionized_metrics::PaymentSuccessRate
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentCount => {
                sessionized_metrics::PaymentCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentSuccessCount => {
                sessionized_metrics::PaymentSuccessCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedPaymentProcessedAmount => {
                sessionized_metrics::PaymentProcessedAmount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedAvgTicketSize => {
                sessionized_metrics::AvgTicketSize
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedRetriesCount => {
                sessionized_metrics::RetriesCount
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::SessionizedConnectorSuccessRate => {
                sessionized_metrics::ConnectorSuccessRate
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::PaymentsDistribution => {
                sessionized_metrics::PaymentsDistribution
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
            Self::FailureReasons => {
                sessionized_metrics::FailureReasons
                    .load_metrics(dimensions, auth, filters, granularity, time_range, pool)
                    .await
            }
        }
    }
}
