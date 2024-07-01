use api_models::{
    analytics::{
        payment_intents::{
            PaymentIntentDimensions, PaymentIntentFilters, PaymentIntentMetricsBucketIdentifier,
        },
        Granularity, TimeRange,
    },
    enums::IntentStatus,
};
use common_utils::errors::ReportSwitchExt;
use error_stack::ResultExt;
use time::PrimitiveDateTime;

use super::PaymentIntentMetricRow;
use crate::{
    query::{
        Aggregate, FilterTypes, GroupByClause, QueryBuilder, QueryFilter, SeriesBucket, ToSql,
        Window,
    },
    types::{AnalyticsCollection, AnalyticsDataSource, MetricsError, MetricsResult},
};

#[derive(Default)]
pub(super) struct SmartRetriedAmount;

#[async_trait::async_trait]
impl<T> super::PaymentIntentMetric<T> for SmartRetriedAmount
where
    T: AnalyticsDataSource + super::PaymentIntentMetricAnalytics,
    PrimitiveDateTime: ToSql<T>,
    AnalyticsCollection: ToSql<T>,
    Granularity: GroupByClause<T>,
    Aggregate<&'static str>: ToSql<T>,
    Window<&'static str>: ToSql<T>,
{
    async fn load_metrics(
        &self,
        dimensions: &[PaymentIntentDimensions],
        merchant_id: &str,
        filters: &PaymentIntentFilters,
        granularity: &Option<Granularity>,
        time_range: &TimeRange,
        pool: &T,
    ) -> MetricsResult<Vec<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>> {
        let mut query_builder: QueryBuilder<T> =
            QueryBuilder::new(AnalyticsCollection::PaymentIntent);

        for dim in dimensions.iter() {
            query_builder.add_select_column(dim).switch()?;
        }
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

        filters.set_filter_clause(&mut query_builder).switch()?;

        query_builder
            .add_filter_clause("merchant_id", merchant_id)
            .switch()?;
        query_builder
            .add_custom_filter_clause("attempt_count", "1", FilterTypes::Gt)
            .switch()?;
        query_builder
            .add_custom_filter_clause("status", IntentStatus::Succeeded, FilterTypes::Equal)
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

        if let Some(granularity) = granularity.as_ref() {
            granularity
                .set_group_by_clause(&mut query_builder)
                .attach_printable("Error adding granularity")
                .switch()?;
        }

        query_builder
            .execute_query::<PaymentIntentMetricRow, _>(pool)
            .await
            .change_context(MetricsError::QueryBuildingError)?
            .change_context(MetricsError::QueryExecutionFailure)?
            .into_iter()
            .map(|i| {
                Ok((
                    PaymentIntentMetricsBucketIdentifier::new(
                        i.status.as_ref().map(|i| i.0),
                        i.currency.as_ref().map(|i| i.0),
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
                Vec<(PaymentIntentMetricsBucketIdentifier, PaymentIntentMetricRow)>,
                crate::query::PostProcessingError,
            >>()
            .change_context(MetricsError::PostProcessingFailure)
    }
}
