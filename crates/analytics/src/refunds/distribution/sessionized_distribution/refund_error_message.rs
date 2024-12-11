use api_models::analytics::{
    refunds::{RefundDimensions, RefundFilters, RefundMetricsBucketIdentifier},
    Granularity, RefundDistributionBody, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::{RefundDistribution, RefundDistributionRow};
use crate::{
    enums::AuthInfo,
    query::{
        Aggregate, GroupByClause, Order, QueryBuilder, QueryFilter, SeriesBucket, ToSql, Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(crate) struct RefundErrorMessage;

#[async_trait::async_trait]
impl<T> RefundDistribution<T> for RefundErrorMessage
where
    T: AnalyticsDataSource + super::RefundDistributionAnalytics,
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
        let mut query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::RefundSessionized);

        for dim in dimensions.iter() {
            query_builder.add_select_column(dim).switch()?;
        }

        query_builder
            .add_select_column(&distribution.distribution_for)
            .switch()?;

        query_builder
            .add_select_column(Aggregate::Count {
                field: None,
                alias: Some("count"),
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

        filters.set_filter_clause(&mut query_builder).switch()?;

        auth.set_filter_clause(&mut query_builder).switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;

        query_builder
            .add_filter_clause(
                RefundDimensions::RefundStatus,
                storage_enums::RefundStatus::Failure,
            )
            .switch()?;

        for dim in dimensions.iter() {
            query_builder
                .add_group_by_clause(dim)
                .attach_printable("Error grouping by dimensions")
                .switch()?;
        }

        query_builder
            .add_group_by_clause(&distribution.distribution_for)
            .attach_printable("Error grouping by distribution_for")
            .switch()?;

        if let Some(granularity) = granularity.as_ref() {
            granularity
                .set_group_by_clause(&mut query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        for dim in dimensions.iter() {
            query_builder.add_outer_select_column(dim).switch()?;
        }

        query_builder
            .add_outer_select_column(&distribution.distribution_for)
            .switch()?;
        query_builder.add_outer_select_column("count").switch()?;
        query_builder
            .add_outer_select_column("start_bucket")
            .switch()?;
        query_builder
            .add_outer_select_column("end_bucket")
            .switch()?;
        let sql_dimensions = query_builder.transform_to_sql_values(dimensions).switch()?;

        query_builder
            .add_outer_select_column(Window::Sum {
                field: "count",
                partition_by: Some(sql_dimensions),
                order_by: None,
                alias: Some("total"),
            })
            .switch()?;

        query_builder
            .add_top_n_clause(
                dimensions,
                distribution.distribution_cardinality.into(),
                "count",
                Order::Descending,
            )
            .switch()?;

        query_builder
            .execute_query::<RefundDistributionRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    RefundMetricsBucketIdentifier::new(
                        i.currency.as_ref().map(|i| i.0),
                        i.refund_status.as_ref().map(|i| i.0.to_string()),
                        i.connector.clone(),
                        i.refund_type.as_ref().map(|i| i.0.to_string()),
                        i.profile_id.clone(),
                        i.refund_reason.clone(),
                        i.refund_error_message.clone(),
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
            .collect::<error_stack::Result<
                Vec<(RefundMetricsBucketIdentifier, RefundDistributionRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
