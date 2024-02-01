use api_models::analytics::{
    api_event::{
        ApiEventDimensions, ApiEventFilters, ApiEventMetrics, ApiEventMetricsBucketIdentifier,
    },
    Granularity, TimeRange,
};
use time::PrimitiveDateTime;

use crate::{
    query::{Aggregate, GroupByClause, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, LoadRow, MetricsResult},
};

mod api_count;
pub mod latency;
mod status_code_count;
use api_count::ApiCount;
use latency::MaxLatency;
use status_code_count::StatusCodeCount;

use self::latency::LatencyAvg;

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ApiEventMetricRow {
    pub latency: Option<u64>,
    pub api_count: Option<u64>,
    pub status_code_count: Option<u64>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}

pub trait ApiEventMetricAnalytics: LoadRow<ApiEventMetricRow> + LoadRow<LatencyAvg> {}

#[async_trait::async_trait]
pub trait ApiEventMetric<T>
where
    T: AnalyticsDataSource + ApiEventMetricAnalytics,
{
    async fn load_metrics(
        &self,
        dimensions: &[ApiEventDimensions],
        merchant_id: &str,
        filters: &ApiEventFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(ApiEventMetricsBucketIdentifier, ApiEventMetricRow)>>;
}

#[async_trait::async_trait]
impl<T> ApiEventMetric<T> for ApiEventMetrics
where
    T: AnalyticsDataSource + ApiEventMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
        /// Asynchronously loads metrics based on the specified dimensions, merchant ID, filters, granularity, time range, and connection pool.
    async fn load_metrics(
        &self,
        dimensions: &[ApiEventDimensions],
        merchant_id: &str,
        filters: &ApiEventFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(ApiEventMetricsBucketIdentifier, ApiEventMetricRow)>> {
        match self {
            Self::Latency => {
                MaxLatency
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
            Self::ApiCount => {
                ApiCount
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
            Self::StatusCodeCount => {
                StatusCodeCount
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
