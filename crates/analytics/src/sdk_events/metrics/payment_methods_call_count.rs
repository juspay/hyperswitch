use api_models::analytics::{
    sdk_events::{
        SdkEventDimensions, SdkEventFilters, SdkEventMetricsBucketIdentifier, SdkEventNames,
    },
    Granularity, TimeRange,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::SdkEventMetricRow;
use crate::{
    query::{Aggregate, GroupByClause, QueryBuilder, QueryFilter, ToSql, Window},
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct PaymentMethodsCallCount;

#[async_trait::async_trait]
impl<T> super::SdkEventMetric<T> for PaymentMethodsCallCount
where
    T: AnalyticsDataSource + super::SdkEventMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
        /// Asynchronously loads the metrics for SDK events based on the provided dimensions, filters, granularity, time range, and database connection pool.
    /// 
    /// # Arguments
    /// 
    /// * `dimensions` - The dimensions to be used for grouping and selecting the metrics.
    /// * `publishable_key` - The publishable key used for filtering the merchant's events.
    /// * `filters` - The event filters to be applied to the query.
    /// * `granularity` - The optional granularity for the metrics aggregation.
    /// * `time_range` - The time range for which the metrics should be loaded.
    /// * `pool` - The database connection pool for executing the query.
    ///
    /// # Returns
    /// 
    /// A `MetricsResult` containing a vector of tuples, where each tuple consists of the bucket identifier and metric row for the SDK events.
    ///
    async fn load_metrics(
        &self,
        dimensions: &[SdkEventDimensions],
        publishable_key: &str,
        filters: &SdkEventFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(SdkEventMetricsBucketIdentifier, SdkEventMetricRow)>> {
        let mut query_builder: QueryBuilder<T> = QueryBuilder::new(AnalyticsCollection::SdkEvents);
        let dimensions = dimensions.to_vec();

        for dim in dimensions.iter() {
            query_builder.add_select_column(dim).switch()?;
        }

        query_builder
            .add_select_column(Aggregate::Count {
                field: None,
                alias: Some("count"),
            })
            .switch()?;

        if let Some(granularity) = granularity.as_ref() {
            query_builder
                .add_granularity_in_mins(granularity)
                .switch()?;
        }

        filters.set_filter_clause(&mut query_builder).switch()?;

        query_builder
            .add_filter_clause("merchant_id", publishable_key)
            .switch()?;

        query_builder
            .add_bool_filter_clause("first_event", 1)
            .switch()?;

        query_builder
            .add_filter_clause("event_name", SdkEventNames::PaymentMethodsCall)
            .switch()?;

        query_builder
            .add_filter_clause("log_type", "INFO")
            .switch()?;

        query_builder
            .add_filter_clause("category", "API")
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

        if let Some(_granularity) = granularity.as_ref() {
            query_builder
                .add_group_by_clause("time_bucket")
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        query_builder
            .execute_query::<SdkEventMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    SdkEventMetricsBucketIdentifier::new(
                        i.payment_method.clone(),
                        i.platform.clone(),
                        i.browser_name.clone(),
                        i.source.clone(),
                        i.component.clone(),
                        i.payment_experience.clone(),
                        i.time_bucket.clone(),
                    ),
                    i,
                ))
            })
            .collect::<error_stack::Result<
                Vec<(SdkEventMetricsBucketIdentifier, SdkEventMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
