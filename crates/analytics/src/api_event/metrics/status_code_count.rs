use api_models::analytics::{
    api_event::{ApiEventDimensions, ApiEventFilters, ApiEventMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::ApiEventMetricRow;
use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, SeriesBucket, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct StatusCodeCount;

#[async_trait::async_trait]
impl<T> super::ApiEventMetric<T> for StatusCodeCount
where
    T: AnalyticsDataSource + super::ApiEventMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
        /// Asynchronously loads and retrieves metrics data based on the specified dimensions, merchant ID, filters, granularity, time range, and database connection pool.
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
            .add_select_column(Aggregate::Count {
                field: Some("status_code"),
                alias: Some("status_code_count"),
            })
            .switch()?;

        filters.set_filter_clause(&mut query_builder).switch()?;

        query_builder
            .add_filter_clause("merchant_id", merchant_id)
            .switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
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

        query_builder
            .execute_query::<ApiEventMetricRow, _>(pool)
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
                    i,
                ))
            })
            .collect::<error_stack::Result<
                Vec<(ApiEventMetricsBucketIdentifier, ApiEventMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
