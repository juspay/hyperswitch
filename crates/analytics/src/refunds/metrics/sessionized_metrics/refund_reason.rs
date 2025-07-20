use std::collections::HashSet;

use api_models::analytics::{
    refunds::{RefundDimensions, RefundFilters, RefundMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::RefundMetricRow;
use crate::{
    enums::AuthInfo,
    query::{
        Aggregate, FilterTypes, GroupByClause, Order, QueryBuilder, QueryFilter, SeriesBucket,
        ToSql, Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(crate) struct RefundReason;

#[async_trait::async_trait]
impl<T> super::RefundMetric<T> for RefundReason
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
        granularity: Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<HashSet<(RefundMetricsBucketIdentifier, RefundMetricRow)>> {
        let mut inner_query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::RefundSessionized);
        inner_query_builder
            .add_select_column("sum(sign_flag)")
            .switch()?;

        inner_query_builder
            .add_custom_filter_clause(
                RefundDimensions::RefundReason,
                "NULL",
                FilterTypes::IsNotNull,
            )
            .switch()?;

        time_range
            .set_filter_clause(&mut inner_query_builder)
            .attach_printable("Error filtering time range for inner query")
            .switch()?;

        let inner_query_string = inner_query_builder
            .build_query()
            .attach_printable("Error building inner query")
            .change_context(MetricsError::QueryBuildingError)?;

        let mut outer_query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::RefundSessionized);

        for dim in dimensions.iter() {
            outer_query_builder.add_select_column(dim).switch()?;
        }

        outer_query_builder
            .add_select_column("sum(sign_flag) AS count")
            .switch()?;

        outer_query_builder
            .add_select_column(format!("({inner_query_string}) AS total"))
            .switch()?;

        outer_query_builder
            .add_select_column(Aggregate::Min {
                field: "created_at",
                alias: Some("start_bucket"),
            })
            .switch()?;

        outer_query_builder
            .add_select_column(Aggregate::Max {
                field: "created_at",
                alias: Some("end_bucket"),
            })
            .switch()?;

        filters
            .set_filter_clause(&mut outer_query_builder)
            .switch()?;

        auth.set_filter_clause(&mut outer_query_builder).switch()?;

        time_range
            .set_filter_clause(&mut outer_query_builder)
            .attach_printable("Error filtering time range for outer query")
            .switch()?;

        outer_query_builder
            .add_custom_filter_clause(
                RefundDimensions::RefundReason,
                "NULL",
                FilterTypes::IsNotNull,
            )
            .switch()?;

        for dim in dimensions.iter() {
            outer_query_builder
                .add_group_by_clause(dim)
                .attach_printable("Error grouping by dimensions")
                .switch()?;
        }

        if let Some(granularity) = granularity {
            granularity
                .set_group_by_clause(&mut outer_query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        outer_query_builder
            .add_order_by_clause("count", Order::Descending)
            .attach_printable("Error adding order by clause")
            .switch()?;

        let filtered_dimensions: Vec<&RefundDimensions> = dimensions
            .iter()
            .filter(|&&dim| dim != RefundDimensions::RefundReason)
            .collect();

        for dim in &filtered_dimensions {
            outer_query_builder
                .add_order_by_clause(*dim, Order::Ascending)
                .attach_printable("Error adding order by clause")
                .switch()?;
        }

        outer_query_builder
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
                HashSet<(RefundMetricsBucketIdentifier, RefundMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
