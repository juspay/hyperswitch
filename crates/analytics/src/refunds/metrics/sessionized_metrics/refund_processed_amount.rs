use std::collections::HashSet;

use api_models::analytics::{
    refunds::{RefundDimensions, RefundFilters, RefundMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::RefundMetricRow;
use crate::{
    enums::AuthInfo,
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, SeriesBucket, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};
#[derive(Default)]
pub(crate) struct RefundProcessedAmount {}

#[async_trait::async_trait]
impl<T> super::RefundMetric<T> for RefundProcessedAmount
where
    T: AnalyticsDataSource + super::RefundMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[RefundDimensions],
        auth: &AuthInfo,
        filters: &RefundFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(RefundMetricsBucketIdentifier, RefundMetricRow)>>
    where
        T: AnalyticsDataSource + super::RefundMetricAnalytics,
    {
        let mut query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::RefundSessionized);

        for dim in dimensions.iter() {
            query_builder.add_select_column(dim).switch()?;
        }

        query_builder
            .add_select_column(Aggregate::Sum {
                field: "refund_amount",
                alias: Some("total"),
            })
            .switch()?;
        query_builder.add_select_column("currency").switch()?;
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

        filters.set_filter_clause(&mut query_builder).switch()?;

        auth.set_filter_clause(&mut query_builder).switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;

        for dim in dimensions.iter() {
            query_builder.add_group_by_clause(dim).switch()?;
        }

        query_builder.add_group_by_clause("currency").switch()?;

        if let Some(granularity) = granularity.as_ref() {
            granularity
                .set_group_by_clause(&mut query_builder)
                .switch()?;
        }

        query_builder
            .add_filter_clause(
                RefundDimensions::RefundStatus,
                storage_enums::RefundStatus::Success,
            )
            .switch()?;

        query_builder
            .execute_query::<RefundMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    RefundMetricsBucketIdentifier::new(
                        i.currency.as_ref().map(|i| i.0),
                        None,
                        i.connector.clone(),
                        i.refund_type.as_ref().map(|i| i.0.to_string()),
                        i.profile_id.clone(),
                        TimeRange {
                            start_time: match (granularity, i.start_bucket) {
                                (Some(g), Some(st)) => g.clip_to_start(st)?,
                                _ => time_range.start_time,
                            },
                            end_time: granularity.as_ref().map_or_else(
                                || Ok(time_range.end_time),
                                |g| i.end_bucket.map(|et| g.clip_to_end(et)).transpose(),
                            )?,
                        },
                    ),
                    i,
                ))
            })
            .collect::<error_stack::Result<HashSet<_>, crate::query::PostProcessingError>>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
