use api_models::analytics::{
    payments::{
        PaymentDimensions, PaymentDistributions, PaymentFilters, PaymentMetricsBucketIdentifier,
    },
    Distribution, Granularity, TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};

mod payment_error_message;

use payment_error_message::PaymentErrorMessage;

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct PaymentDistributionRow {
    pub currency: Option<DBEnumWrapper<storage_enums::Currency>>,
    pub status: Option<DBEnumWrapper<storage_enums::AttemptStatus>>,
    pub connector: Option<String>,
    pub authentication_type: Option<DBEnumWrapper<storage_enums::AuthenticationType>>,
    pub payment_method: Option<String>,
    pub payment_method_type: Option<String>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    pub error_message: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait PaymentDistributionAnalytics: LoadRow<PaymentDistributionRow> {}

#[async_trait::async_trait]
pub trait PaymentDistribution<T>
where
    T: AnalyticsDataSource + PaymentDistributionAnalytics,
{
    #[allow(clippy::too_many_arguments)]
    async fn load_distribution(
        &self,
        distribution: &Distribution,
        dimensions: &[PaymentDimensions],
        merchant_id: &str,
        filters: &PaymentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentMetricsBucketIdentifier, PaymentDistributionRow)>>;
}

#[async_trait::async_trait]
impl<T> PaymentDistribution<T> for PaymentDistributions
where
    T: AnalyticsDataSource + PaymentDistributionAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
        /// Asynchronously loads the distribution of payments based on the provided parameters.
    async fn load_distribution(
        &self,
        distribution: &Distribution,
        dimensions: &[PaymentDimensions],
        merchant_id: &str,
        filters: &PaymentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentMetricsBucketIdentifier, PaymentDistributionRow)>> {
        match self {
            Self::PaymentErrorMessage => {
                PaymentErrorMessage
                    .load_distribution(
                        distribution,
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
