use api_models::analytics::{
    api_event::{ApiEventDimensions, ApiEventFilters, ApiEventMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::ApiEventMetricRow;
use crate::{
    query::{
        Aggregate, FilterTypes, GroupByClause, QueryBuilder, QueryFilter, SeriesBucket, ToSql,
        Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct MaxLatency;

#[async_trait::async_trait]
impl<T> super::ApiEventMetric<T> for MaxLatency
where
    T: AnalyticsDataSource + super::ApiEventMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        _dimensions: &[ApiEventDimensions],
        merchant_id: &str,
        filters: &ApiEventFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(ApiEventMetricsBucketIdentifier, ApiEventMetricRow)>> {
        let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::ApiEvents);

        query_builder
            .add_select_column(Aggregate::Sum {
                field: "latency",
                alias: Some("latency_sum"),
            })
            .switch()?;

        query_builder
            .add_select_column(Aggregate::Count {
                field: Some("latency"),
                alias: Some("latency_count"),
            })
            .switch()?;

        query_builder
            .add_select_column(Aggregate::Min {
                field: "created_at",
                alias: Some("start_bucket"),
            })
            .switch()?;
        query_builder
            .add_select_column(Aggregate::Max {
                field: "created_at",
                alias: Some("end_bucket"),
            })
            .switch()?;
        if let Some(granularity) = granularity.as_ref() {
            granularity
                .set_group_by_clause(&mut query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        filters.set_filter_clause(&mut query_builder).switch()?;

        query_builder
            .add_filter_clause("merchant_id", merchant_id)
            .switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;

        query_builder
            .add_custom_filter_clause("request", "10.63.134.6", FilterTypes::NotLike)
            .attach_printable("Error filtering out locker IP")
            .switch()?;

        query_builder
            .execute_query::<LatencyAvg, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    ApiEventMetricsBucketIdentifier::new(TimeRange {
                        start_time: match (granularity, i.start_bucket) {
                            (Some(g), Some(st)) => g.clip_to_start(st)?,
                            _ => time_range.start_time,
                        },
                        end_time: granularity.as_ref().map_or_else(
                            || Ok(time_range.end_time),
                            |g| i.end_bucket.map(|et| g.clip_to_end(et)).transpose(),
                        )?,
                    }),
                    ApiEventMetricRow {
                        latency: if i.latency_count != 0 {
                            Some(i.latency_sum.unwrap_or(0) / i.latency_count)
                        } else {
                            None
                        },
                        api_count: None,
                        status_code_count: None,
                        start_bucket: i.start_bucket,
                        end_bucket: i.end_bucket,
                    },
                ))
            })
            .collect::<error_stack::Result<
                Vec<(ApiEventMetricsBucketIdentifier, ApiEventMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub struct LatencyAvg {
    latency_sum: Option<u64>,
    latency_count: u64,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub start_bucket: Option<PrimitiveDateTime>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub end_bucket: Option<PrimitiveDateTime>,
}
