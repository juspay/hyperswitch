use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetricsBucketIdentifier},
    Distribution, Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use diesel_models::enums as storage_enums;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::{PaymentDistribution, PaymentDistributionRow};
use crate::{
    query::{
        Aggregate, GroupByClause, Order, QueryBuilder, QueryFilter, SeriesBucket, ToSql, Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct PaymentErrorMessage;

#[async_trait::async_trait]
impl<T> PaymentDistribution<T> for PaymentErrorMessage
where
    T: AnalyticsDataSource + super::PaymentDistributionAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_distribution(
        &self,
        distribution: &Distribution,
        dimensions: &[PaymentDimensions],
        merchant_ids: &[String],
        filters: &PaymentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentMetricsBucketIdentifier, PaymentDistributionRow)>> {
        let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::Payment);

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

        query_builder
            .add_filter_in_range_clause("merchant_id", merchant_ids)
            .switch()?;

        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
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

        query_builder
            .add_filter_clause(
                PaymentDimensions::PaymentStatus,
                storage_enums::AttemptStatus::Failure,
            )
            .switch()?;

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
            .execute_query::<PaymentDistributionRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    PaymentMetricsBucketIdentifier::new(
                        i.currency.as_ref().map(|i| i.0),
                        i.status.as_ref().map(|i| i.0),
                        i.connector.clone(),
                        i.authentication_type.as_ref().map(|i| i.0),
                        i.payment_method.clone(),
                        i.payment_method_type.clone(),
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
                Vec<(PaymentMetricsBucketIdentifier, PaymentDistributionRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
