use api_models::analytics::{
    refunds::{
        RefundDimensions, RefundDistributions, RefundFilters, RefundMetricsBucketIdentifier,
        RefundType,
    },
    Granularity, RefundDistributionBody, TimeRange,
};
use diesel_models::enums as storage_enums;
use time::PrimitiveDateTime;

use crate::{
    enums::AuthInfo,
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, DBEnumWrapper, LoadRow, MetricsResult},
};

mod sessionized_distribution;

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct RefundDistributionRow {
    pub currency: Option<DBEnumWrapper<storage_enums::Currency>>,
    pub refund_status: Option<DBEnumWrapper<storage_enums::RefundStatus>>,
    pub connector: Option<String>,
    pub refund_type: Option<DBEnumWrapper<RefundType>>,
    pub profile_id: Option<String>,
    pub total: Option<bigdecimal::BigDecimal>,
    pub count: Option<i64>,
    pub refund_reason: Option<String>,
    pub refund_error_message: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait RefundDistributionAnalytics: LoadRow<RefundDistributionRow> {}

#[async_trait::async_trait]
pub trait RefundDistribution<T>
where
    T: AnalyticsDataSource + RefundDistributionAnalytics,
{
    #[allow(clippy::too_many_arguments)]
    async fn load_distribution(
        &self,
        distribution: &RefundDistributionBody,
        dimensions: &[RefundDimensions],
        auth: &AuthInfo,
        filters: &RefundFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(RefundMetricsBucketIdentifier, RefundDistributionRow)>>;
}

#[async_trait::async_trait]
impl<T> RefundDistribution<T> for RefundDistributions
where
    T: AnalyticsDataSource + RefundDistributionAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_distribution(
        &self,
        distribution: &RefundDistributionBody,
        dimensions: &[RefundDimensions],
        auth: &AuthInfo,
        filters: &RefundFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(RefundMetricsBucketIdentifier, RefundDistributionRow)>> {
        match self {
            Self::SessionizedRefundReason => {
                sessionized_distribution::RefundReason
                    .load_distribution(
                        distribution,
                        dimensions,
                        auth,
                        filters,
                        granularity,
                        time_range,
                        pool,
                    )
                    .await
            }
            Self::SessionizedRefundErrorMessage => {
                sessionized_distribution::RefundErrorMessage
                    .load_distribution(
                        distribution,
                        dimensions,
                        auth,
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
