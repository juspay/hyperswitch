use api_models::analytics::{
    payments::{PaymentDimensions, PaymentFilters, PaymentMetricsBucketIdentifier},
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::PaymentMetricRow;
use crate::{
    query::{
        Aggregate, FilterTypes, GroupByClause, QueryBuilder, QueryFilter, SeriesBucket, ToSql,
        Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct RetriesCount;

#[async_trait::async_trait]
impl<T> super::PaymentMetric<T> for RetriesCount
where
    T: AnalyticsDataSource + super::PaymentMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
        /// Asynchronously loads payment metrics based on the provided dimensions, merchant ID, filters, granularity, time range, and database pool. It constructs a query, adds select columns and filter clauses, executes the query, and processes the results into a vector of payment metric rows along with their corresponding bucket identifiers. Returns a `MetricsResult` containing the vector of payment metric rows and their bucket identifiers, or an error if the query building, execution, or post-processing fails.
    async fn load_metrics(
        &self,
        _dimensions: &[PaymentDimensions],
        merchant_id: &str,
        _filters: &PaymentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>> {
        let mut query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::PaymentIntent);
        query_builder
            .add_select_column(Aggregate::Count {
                field: None,
                alias: Some("count"),
            })
            .switch()?;
        query_builder
            .add_select_column(Aggregate::Sum {
                field: "amount",
                alias: Some("total"),
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
        query_builder
            .add_filter_clause("merchant_id", merchant_id)
            .switch()?;
        query_builder
            .add_custom_filter_clause("attempt_count", "1", FilterTypes::Gt)
            .switch()?;
        query_builder
            .add_custom_filter_clause("status", "succeeded", FilterTypes::Equal)
            .switch()?;
        time_range
            .set_filter_clause(&mut query_builder)
            .attach_printable("Error filtering time range")
            .switch()?;

        if let Some(granularity) = granularity.as_ref() {
            granularity
                .set_group_by_clause(&mut query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        query_builder
            .execute_query::<PaymentMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    PaymentMetricsBucketIdentifier::new(
                        i.currency.as_ref().map(|i| i.0),
                        None,
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
                Vec<(PaymentMetricsBucketIdentifier, PaymentMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
